// this is hekkin terrible, and ideally you should use like a proper grammar or something
// actually hm.. we have a crate so maybe..

trait NodeExt {
    fn as_text(&self) -> &str;
}
impl NodeExt for parse_wiki_text::Node<'_> {
    fn as_text(&self) -> &str {
        match self {
            parse_wiki_text::Node::Text { value, .. } => value,
            _ => panic!("{:?} is not a text node", self),
        }
    }
}

pub fn update_infobox<S1, S2>(page_text: &str, new_ibox_values: &[(S1, S2)]) -> String
where
    S1: AsRef<str> + std::fmt::Debug,
    S2: AsRef<str> + std::fmt::Debug,
{
    let parsed = parse_wiki_text::Configuration::default().parse(page_text);
    assert!(parsed.warnings.is_empty());

    let mut infoboxen = vec![];
    for node in parsed.nodes {
        match node {
            parse_wiki_text::Node::Template {
                start,
                end,
                name,
                parameters,
            } => {
                assert_eq!(name.len(), 1);
                match name[0] {
                    parse_wiki_text::Node::Text { value, .. } => {
                        if value.to_lowercase() == "item" {
                            infoboxen.push((start, end, parameters))
                        }
                    }
                    _ => panic!("{:?}", name),
                }
            }
            _ => {}
        }
    }

    assert_eq!(infoboxen.len(), 1);
    // sanity check
    // yes, quadratic
    for (i, v1) in new_ibox_values.iter().enumerate() {
        for v2 in &new_ibox_values[i + 1..] {
            if v1.0.as_ref().to_lowercase() == v2.0.as_ref().to_lowercase() {
                panic!("duplicate in-values: {:?} {:?}", v1, v2)
            }
        }
    }

    let ibox = &infoboxen[0];
    let mut updated_parts = vec![];
    let mut used_keys = std::collections::HashSet::new();
    for param in &ibox.2 {
        let name = param.name.as_ref().unwrap();
        assert_eq!(name.len(), 1);
        let name = name[0].as_text();
        let old_param_str = &page_text[param.start..param.end];
        let new_val: Option<&str> = new_ibox_values
            .iter()
            .find(|(n, _)| n.as_ref().to_lowercase() == name.to_lowercase())
            .map(|x| x.1.as_ref().trim());

        used_keys.insert(name);

        match new_val {
            None => {
                updated_parts.push(old_param_str.to_string());
            }
            Some(new_val) => {
                updated_parts.push(format!("{} = {}", name, new_val));
            }
        }
    }

    let new_ibox = _make_infobox_skipping_some(new_ibox_values, updated_parts, Some(&used_keys));
    let new_page = page_text[..ibox.0].to_string() + &new_ibox + &page_text[ibox.1..];

    new_page
}

pub fn write_new_page(new_ibox_values: &[(&str, String)]) -> String {
    let ibox = _make_infobox_skipping_some(new_ibox_values, vec![], None);
    let page = ibox + "\n{{Stub}}";
    page
}

fn _make_infobox_skipping_some<S1, S2>(
    new_ibox_values: &[(S1, S2)],
    mut pre_existing_part: Vec<String>,
    skip_these_keys: Option<&std::collections::HashSet<&str>>,
) -> String
where
    S1: AsRef<str> + std::fmt::Debug,
    S2: AsRef<str> + std::fmt::Debug,
{
    let empty = std::collections::HashSet::default();
    let skip_these = skip_these_keys.unwrap_or(&empty);

    for (k, v) in new_ibox_values {
        let k = k.as_ref();
        let v = v.as_ref().trim();
        if skip_these.contains(&k) {
            continue;
        }
        pre_existing_part.push(format!("{} = {}", k, v));
    }

    let tmp = pre_existing_part
        .iter()
        .map(|x| format!(" | {}", x))
        .collect::<Vec<_>>();
    let new_ibox = "{{Item\n".to_string() + &tmp.join("\n") + "\n}}";
    new_ibox
}
