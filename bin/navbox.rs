use std::{collections::HashMap, io::Write};

use clap::Parser;
#[derive(Parser, Debug)]
struct Args {
    #[clap(long)]
    data: String,
    // #[clap(long)]
    // out: String
}

fn main() {
    let args = Args::parse();
    let db = backpack_wiki::backpack_db::load_db(&args.data);
    let r_names = vec!["Common", "Uncommon", "Rare", "Legendary"];
    let cat_other = "Other";
    let cats = vec![
        ("Weapon", vec!["Weapon", "Bow", "Wand"]),
        ("Armor", vec!["Armor", "Shield", "Structure"]),
        ("Accessory", vec!["Accessory", "Ingredient"]),
        ("Consumable", vec!["Consumable"]),
        ("Curse", vec!["Curse"]),
        ("Relic", vec!["Relic"]),
        (cat_other, vec![]),
    ];

    let mut categorized = HashMap::<_, Vec<_>>::new();
    // let mut rarities = r_names
    //     .iter()
    //     .map(|x| (*x, vec![]))
    //     .collect::<HashMap<_, Vec<&Item>>>();
    for item in &db.items {
        let catname = cats
            .iter()
            .filter(|(_catname, types)| types.iter().any(|t| item.types.contains(&t.to_string())))
            .map(|(catname, _)| catname)
            .collect::<Vec<_>>();
        assert!(catname.len() <= 1);
        let catname = match catname.first() {
            Some(c) => c,
            None => cat_other,
        };
        assert!(r_names.contains(&item.rarity.as_str()));
        categorized
            .entry((catname.to_string(), item.rarity.to_string()))
            .or_default()
            .push(item);
    }

    // :pensive:
    // i should have just went with a tera or something...
    let mut output = String::new();
    for (cat, _) in cats.iter() {
        let rarity_row_count = categorized.keys().filter(|(c, _)| c == cat).count();
        if rarity_row_count == 1 {
            assert!(["Curse", "Relic", cat_other].contains(cat));
        }

        output.push_str("|-\n");
        output.push_str(&format!(
            "! width=\"10%\" rowspan=\"{}\" | {}\n",
            rarity_row_count, cat
        ));

        let mut rows_here = vec![];
        for rarity in &r_names {
            let mut row = String::new();
            let entries = match categorized.get(&(cat.to_string(), rarity.to_string())) {
                Some(e) => e,
                None => continue,
            };

            let mut itemnames = entries.iter().map(|i| i.name.as_str()).collect::<Vec<_>>();
            itemnames.sort();
            let itemnames = itemnames
                .into_iter()
                .map(|i| format!("[[{}]]", i))
                .collect::<Vec<_>>();

            if rarity_row_count == 1 {
                row.push_str("| colspan=\"2\" ");
            } else {
                row.push_str(&format!("! {}\n", rarity));
                // row.push_str("| ");
            }
            row.push_str(&format!("| {} \n", itemnames.join(" • ")));
            rows_here.push(row);
        }
        output.push_str(&rows_here.join("|-\n"));
    }

    //
    // for &r in r_names.iter() {
    //     output.push_str(&format!("|-\n! width=\"10%\" | {}\n", r));
    //     let items = rarities.get_mut(r).unwrap();
    //     items.sort_by_key(|i| &i.name);
    //     let items_links = items
    //         .iter()
    //         .map(|x| format!("[[{}]]", x.name))
    //         .collect::<Vec<_>>();
    //     let x = items_links.join(" • ");
    //     output.push_str(&format!("| {}\n", x));
    // }

    let out_fn = std::path::Path::new("out/navbox.txt");
    std::fs::create_dir_all(out_fn.parent().unwrap()).unwrap();
    let mut out_f = std::fs::File::create(out_fn).unwrap();
    out_f.write(output.as_bytes()).unwrap();
}
