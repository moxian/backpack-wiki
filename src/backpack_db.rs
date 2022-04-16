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
struct ItemRaw {
    name: String,
    types: Vec<String>,
    rarity: String,
    descriptors: Vec<String>,
    #[serde(flatten)]
    cost: Option<UseCost>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(from = "ItemRaw")]
pub struct Item {
    pub id: String,
    pub name: String,
    pub types: Vec<String>,
    pub rarity: String,
    pub descriptors: Vec<String>,
    #[serde(flatten)]
    pub cost: Option<UseCost>,
}
impl From<ItemRaw> for Item {
    fn from(f: ItemRaw) -> Self {
        let mut item = Item {
            name: capitalize(&f.name),
            id: f.name,
            types: f.types,
            rarity: f.rarity,
            descriptors: f.descriptors,
            cost: f.cost,
        };
        // because serde doesn't do what i want it to do
        if let Some(cost) = &item.cost {
            if cost.energy.is_none() && cost.mana.is_none() && cost.gold.is_none() {
                item.cost = None
            }
        }
        item
    }
}

fn wikify_description_links(items: &mut [Item]) {
    let mut id_names = items
        .iter()
        .map(|i| (i.id.clone(), i.name.clone()))
        .collect::<Vec<_>>();
    // idk something about data dump not having ethereal one, and giving bad linkification
    if !id_names.iter().any(|x| x.0 == "ETHEREAL ARROW") {
        id_names = id_names
            .into_iter()
            .filter(|(id, _)| id != "ARROW")
            .collect();
    }
    // yes, quadratic, screw it
    for item in items.iter_mut() {
        for line in &mut item.descriptors {
            let mut candidates = vec![];
            for (id, name) in &id_names {
                if line.contains(id) {
                    candidates.push((id, name));
                }
            }
            // aaaaa!!
            // Yes, this breaks on more than one link, i'm well aware
            while candidates.len() > 1 {
                candidates.sort_by_key(|x| x.0.len());
                assert!(candidates
                    .iter()
                    .skip(1)
                    .all(|(c, _): &(&String, _)| c.contains(candidates[0].0)));
                candidates.remove(0);
            }
            assert!(candidates.len() <= 1, "{:?} {:?}", item, candidates);
            if let Some((id, name)) = candidates.first() {
                *line = line.replace(id.as_str(), &format!("[[{}]]", name));
            }
        }
    }
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
        if let Some(cost) = &self.cost {
            let mut words = vec![];
            if let Some(x) = cost.energy {
                words.push(format!("{} Energy", x));
            }
            if let Some(x) = cost.mana {
                words.push(format!("{} mana", x));
            }
            if let Some(x) = cost.gold {
                words.push(format!("{} Gold", x));
            }
            assert!(words.len() != 0, "{:?}", self);
            infobox_parts.push(("useCost", words.join(", ")));
        }
        if !effects.is_empty() {
            infobox_parts.push(("effects", effects.join("<br/>\n")))
        }
        if !description.is_empty() {
            infobox_parts.push(("description", description.join("<br/>\n")))
        }
        infobox_parts
    }
}

// wonderful engineering yesyes
fn capitalize(s: &str) -> String {
    let mut out = "".to_string();
    let mut start = true;
    for (i, c) in s.char_indices() {
        if start && !s[i..].to_lowercase().starts_with("of ") {
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

pub fn load_db(data_path: impl AsRef<std::path::Path>) -> Db {
    let mut db: Db =
        json5::from_str(&std::fs::read_to_string(data_path.as_ref()).unwrap()).unwrap();
    wikify_description_links(&mut db.items);
    // panic!();
    db
}
