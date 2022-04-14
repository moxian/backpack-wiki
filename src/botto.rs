const WIKI_URL: &str = "https://backpack-hero.fandom.com/api.php";

// *** vvv THESE ARE THE KNOBS YOU ARE LOOKING FOR vvv *** //
const PROMPT_EXISTING: bool = true;
const PROMPT_NEW: bool = true;
const EDIT_OK: bool = true; // main knob overriding the two above

#[derive(serde::Deserialize)]
struct Cred {
    name: String,
    password: String,
}

async fn get_existing_page_text(api: &mediawiki::api::Api, page: &str) -> Option<String> {
    let params = api.params_into(&[
        ("action", "parse"),
        ("page", page),
        ("prop", "wikitext"),
        ("formatversion", "2"), //cargo cult
    ]);
    let res = api.post_query_api_json(&params).await.unwrap();
    let err = &res.as_object().unwrap().get("error");
    if let Some(err) = err {
        let code = err.as_object().unwrap()["code"].as_str().unwrap();
        if code == "missingtitle" {
            return None;
        }
        panic!("{:?}", err);
    }
    let text = res.as_object().unwrap()["parse"].as_object().unwrap()["wikitext"]
        .as_str()
        .unwrap()
        .to_string();
    Some(text)
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum MediawikiErrorCode {
    Ratelimited,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)] // yes, i'm mad
struct MediawikiError {
    #[serde(rename = "*")]
    star: String,
    code: MediawikiErrorCode,
    info: String,
}

// ideally i'd parse it into a proper enum but that's looks even more gnarly..
fn extract_mediawiki_error(v: &serde_json::Value) -> Option<MediawikiError> {
    let err: MediawikiError = serde_json::from_value(v.as_object().unwrap().get("error")?.clone())
        .unwrap_or_else(|e| panic!("{}: {:?}", e, v));
    Some(err)
}

pub(crate) async fn stuff() {
    // see https://www.mediawiki.org/wiki/Manual:Bot_passwords on instructions on how to get creds
    let cred: Cred =
        serde_json::from_str(&std::fs::read_to_string("bot-creds.json").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.login(cred.name, cred.password).await.unwrap();
    let token = api.get_edit_token().await.unwrap();

    let db = crate::backpack_db::load_db();
    let version_string = format!("v{}", db.version);

    let update_summary = format!(
        "Mass updating the items from the v{} data dump. Code available at https://github.com/moxian/backpack-wiki",
        db.version
    );

    for item in &db.items {
        let page_name = item.name.as_str();
        println!("{}", page_name);
        let mut infobox_parts = item.to_infobox_pairs();
        infobox_parts.push(("lastUpdate", version_string.clone()));

        let existing_text = get_existing_page_text(&api, page_name).await;
        // println!("existing: {:?}", existing_text);
        let new_page_text = match &existing_text {
            Some(existing_text) => crate::wikiparse::update_infobox(&existing_text, &infobox_parts),
            None => crate::wikiparse::write_new_page(&infobox_parts),
        };

        if Some(&new_page_text) == existing_text.as_ref() {
            println!("  ..no changes");
            continue;
        }

        // println!("page for {:?}:\n{}\n\n", page_name, new_page_text);

        if existing_text.is_some() {
            println!("  .. differs");
        } else {
            println!("  .. does not exist");
        }
        // user prompt
        if existing_text.is_some() && PROMPT_EXISTING || !existing_text.is_some() && PROMPT_NEW {
            if let Some(existing) = &existing_text {
                println!(
                    "Existing:\n{}\n\nProposed:\n{}\n\n Edit? (y/N/q)",
                    existing, new_page_text
                );
            } else {
                println!("  Create? (y/N/q)");
            }

            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer).unwrap();
            match answer.trim().to_lowercase().as_str() {
                "y" | "yes" => {}
                "q" | "quit" => {
                    println!("quitting.");
                    break;
                }
                _ => {
                    println!("skipping");
                    continue;
                }
            }
        }

        if !EDIT_OK {
            continue;
        }
        println!("  .. working...");
        let params = api.params_into(&[
            ("action", "edit"),
            ("title", page_name),
            // ("minor", "true"),
            ("bot", "true"),
            ("text", &new_page_text),
            ("token", &token),
            ("summary", &update_summary),
        ]);

        loop {
            let res = api.post_query_api_json(&params).await.unwrap();
            let err = extract_mediawiki_error(&res);
            if let Some(err) = err {
                if err.code == MediawikiErrorCode::Ratelimited {
                    let wait_for = 70;
                    println!(
                        "We are being ratelimited... Waiting {} seconds before retrying..",
                        wait_for
                    );
                    std::thread::sleep(std::time::Duration::from_secs(wait_for));
                    continue;
                } // else
                println!("Failed to edit: {:#?}", err);
                panic!("{:?}", res);
            }
            assert_eq!(
                res.as_object()
                    .and_then(|x| x.get("edit"))
                    .and_then(|x| x.as_object())
                    .and_then(|x| x.get("result"))
                    .and_then(|x| x.as_str()),
                Some("Success"),
                "{:?}",
                res
            );
            break; // from the retry loop
        }
        // idk if i want to handle the response here honestly... The options are:
        // {"edit": {"new": ""}} // literal empty string, yes
        // {"edit": {"nochange": ""}}
        // {"edit": {"newrevid":443,"newtimestamp":"2022-04-14T02:05:21Z","oldrevid":442}}

        // println!("{}", &res.to_string());
        // break;
    }
}

pub(crate) fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(stuff());
}
