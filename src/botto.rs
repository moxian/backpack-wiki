use crate::Args;

const WIKI_URL: &str = "https://backpack-hero.fandom.com/api.php";

// *** vvv SOME RANDOM KNOBS vvv *** //
const VERSION_BUMP_IS_NOTABLE: bool = false;
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

fn cmp_version(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1 = v1.strip_prefix("v").unwrap();
    let v2 = v2.strip_prefix("v").unwrap();
    std::cmp::PartialOrd::partial_cmp(
        &version_compare::Version::from(v1).unwrap(),
        &version_compare::Version::from(v2).unwrap(),
    )
    .unwrap()
}

pub(crate) async fn stuff(args: &Args) {
    // see https://www.mediawiki.org/wiki/Manual:Bot_passwords on instructions on how to get creds
    let cred: Cred =
        serde_json::from_str(&std::fs::read_to_string("bot-creds.json").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.set_user_agent("backpack-hero wiki botto (by moxian; source: https://github.com/moxian/backpack-wiki)");
    api.set_edit_delay(Some(1000));

    api.login(cred.name, cred.password).await.unwrap();
    let token = api.get_edit_token().await.unwrap();

    let db = crate::backpack_db::load_db(&args.data);
    let version_string = format!("v{}", db.version);

    for item in db.items.iter() {
        let page_name = item.name.as_str();
        // if page_name != "Gold" {
        //     continue;
        // }
        print!("{} ..", page_name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut infobox_parts = item.to_infobox_pairs();
        infobox_parts.push(("lastUpdate", version_string.clone()));

        let existing_text = get_existing_page_text(&api, page_name).await;
        // todo: this probably breaks again on None
        let (new_page_text, notable) = if let Some(existing_text) = &existing_text {
            let mut page = crate::wikiparse::parse_page(&existing_text);
            assert_eq!(page.item_infoboxes.len(), 1);
            let infobox = page.item_infoboxes.iter_mut().next().unwrap();

            let mut notable = false;
            for (k, v) in infobox_parts {
                match infobox.params.entry(k.to_string()) {
                    indexmap::map::Entry::Vacant(e) => {
                        e.insert(v);
                        notable = true;
                    }
                    indexmap::map::Entry::Occupied(mut e) => {
                        if e.get() != &v {
                            e.insert(v);
                            if k != "lastUpdate" && VERSION_BUMP_IS_NOTABLE {
                                notable = true
                            }
                        }
                    }
                }
            }
            let new_page_text = crate::wikiparse::reformat_page(&page);
            (new_page_text, notable)
        } else {
            todo!("handle creation..");
        };

        // // println!("existing: {:?}", existing_text);
        // let new_page_text = match &existing_text {
        //     Some(existing_text) => {
        //         let res = crate::wikiparse::update_infobox(&existing_text, &infobox_parts).unwrap();
        //         if existing_text == &res.new_text {
        //             println!(" .. no change");
        //             continue;
        //         }
        //         if !res.meaningful_change {
        //             println!(" .. not a meaningful change");
        //             continue;
        //         }
        //         if matches!(
        //             cmp_version(res.old_version.as_ref().unwrap(), &version_string),
        //             std::cmp::Ordering::Greater
        //         ) {
        //             println!(" .. refusing to downgrade");
        //             continue;
        //         }
        //         res.new_text
        //     }
        //     None => crate::wikiparse::write_new_page(&infobox_parts),
        // };

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
        if existing_text.is_some() && notable && PROMPT_EXISTING
            || !existing_text.is_some() && PROMPT_NEW
        {
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
        let summary = match &args.summary {
            Some(s) => s,
            None => {
                println!("No `--summary` provided! Not gonna edit stuff!");
                continue;
            }
        };
        println!("  .. working...");
        let params = api.params_into(&[
            ("action", "edit"),
            ("title", page_name),
            ("minor", "true"), // keep this.. Otherwise mass edits look very gnarly in recent changes...
            ("bot", "true"),
            ("text", &new_page_text),
            ("token", &token),
            ("summary", summary),
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

pub(crate) fn main(args: &Args) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(stuff(args));
}
