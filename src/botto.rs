const WIKI_URL: &str = "https://backpack-hero.fandom.com/api.php";

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

pub(crate) async fn stuff() {
    let cred: Cred =
        serde_json::from_str(&std::fs::read_to_string("bot-creds.json").unwrap()).unwrap();
    let mut api = mediawiki::api::Api::new(WIKI_URL).await.unwrap();
    api.login(cred.name, cred.password).await.unwrap();
    let token = api.get_edit_token().await.unwrap();

    let db = crate::backpack_db::load_db();

    let edit_existing = true;
    let create_new = true;
    let edit_ok = false; // main knob overriding the two above
    let update_summary = format!(
        "Mass updating the items from the v{} data dump. Code available at https://github.com/moxian/backpack-wiki",
        db.version
    );

    for item in &db.items {
        let page_name = item.name.as_str();
        println!("{}", page_name);
        let infobox_parts = item.to_infobox_pairs();

        let existing_text = get_existing_page_text(&api, page_name).await;
        let new_page_text = match existing_text {
            Some(existing_text) => {
                if !edit_existing {
                    continue;
                }
                crate::wikiparse::update_infobox(&existing_text, &infobox_parts)
            }
            None => {
                if !create_new {
                    continue;
                }
                crate::wikiparse::write_new_page(&infobox_parts)
            }
        };

        println!("page for {:?}:\n{}\n\n", page_name, new_page_text);

        if edit_ok {
            let params = api.params_into(&[
                ("action", "edit"),
                ("title", page_name),
                ("minor", "true"),
                ("bot", "true"),
                ("text", &new_page_text),
                ("token", &token),
                ("summary", &update_summary),
            ]);
            let res = api.post_query_api_json(&params).await.unwrap();
        }
    }
}

pub(crate) fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(stuff());
}
