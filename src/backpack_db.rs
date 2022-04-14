#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Db {
    pub version: String,
    pub items: Vec<Item>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UseCost {
    energy: Option<i32>,
    mana: Option<i32>,
    gold: Option<i32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub name: String,
    pub types: Vec<String>,
    pub rarity: String,
    pub descriptors: Vec<String>,
    #[serde(flatten)]
    pub cost: Option<UseCost>,
}

impl Item {
    pub fn to_infobox_pairs(&self) -> Vec<(&'static str, String)> {
        let mut effects = vec![];
        let mut description = vec![];
        for d in &self.descriptors {
            let d = d.trim();
            if let Some(stripped) = d.strip_prefix("/") {
                description.push(stripped);
            } else {
                effects.push(d)
            }
        }
        let mut infobox_parts = vec![
            ("title", self.name.to_string()),
            ("type", self.types.join(", ")),
            ("rarity", self.rarity.to_string()),
        ];
        if let Some(cost) = &self.cost{
            let mut words = vec![];
            if let Some(x) = cost.energy{
                words.push(format!("{} Energy", x));
            }
            if let Some(x) = cost.mana{
                words.push(format!("{} mana", x));
            }
            if let Some(x) = cost.gold{
                words.push(format!("{} Gold", x));
            }
            assert!(words.len()!=0, "{:?}", self);
            infobox_parts.push(("useCost", words.join(", ")));
        }
        if !effects.is_empty() {
            infobox_parts.push(("effects", effects.join("\n")))
        }
        if !description.is_empty() {
            infobox_parts.push(("description", description.join("\n")))
        }
        infobox_parts
    }
}

// wonderful engineering yesyes
fn capitalize(s: &str) -> String {
    let mut out = "".to_string();
    let mut start = true;
    for c in s.chars() {
        if start {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        start = false;
        if c == ' ' {
            start = true;
        }
    }
    out
}

pub fn load_db() -> Db {
    let mut db: Db =
        json5::from_str(&std::fs::read_to_string(crate::DATA_DUMP_PATH).unwrap()).unwrap();
    for item in &mut db.items {
        item.name = capitalize(&item.name); // because i don't like allcaps
        // because serde doesn't do what i want it to do

        if let Some(cost) = &item.cost{
            if cost.energy.is_none() && cost.mana.is_none() && cost.gold.is_none() {
                item.cost = None
            }
        }
    }
    db
}
