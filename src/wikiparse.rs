trait NodeExt {
    fn as_text(&self) -> &str;
    fn range(&self) -> std::ops::Range<usize>;
    fn start(&self) -> usize;
    fn end(&self) -> usize;
}
impl NodeExt for parse_wiki_text::Node<'_> {
    fn as_text(&self) -> &str {
        match self {
            parse_wiki_text::Node::Text { value, .. } => value,
            _ => panic!("{:?} is not a text node", self),
        }
    }
    fn range(&self) -> std::ops::Range<usize> {
        use parse_wiki_text::Node;
        match self {
            Node::Text { start, end, .. } => *start..*end,
            Node::Link { start, end, .. } => *start..*end,
            Node::Template { start, end, .. } => *start..*end,
            _ => unimplemented!("Node: {:?}", self),
        }
    }
    fn start(&self) -> usize {
        self.range().start
    }
    fn end(&self) -> usize {
        self.range().end
    }
}

trait ParameterExt {
    fn as_str<'a>(&self, source: &'a str) -> &'a str;
    fn name_str<'a>(&self, source: &'a str) -> &'a str;
    fn val_str<'a>(&self, source: &'a str) -> &'a str;
}
impl ParameterExt for parse_wiki_text::Parameter<'_> {
    fn as_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    fn name_str<'a>(&self, source: &'a str) -> &'a str {
        &source[self.name.as_ref().unwrap().first().unwrap().start()
            ..self.name.as_ref().unwrap().last().unwrap().end()]
    }
    fn val_str<'a>(&'_ self, source: &'a str) -> &'a str {
        if self.value.len() == 0 {
            ""
        } else {
            let first = self.value.first().unwrap();
            let last = self.value.last().unwrap();
            &source[first.start()..last.end()]
        }
    }
}

pub enum TriState {
    Yes,
    #[allow(dead_code)]
    No,
    Idk,
}
// do we have it, and do we want it?
struct Intent<T> {
    have: Option<T>,
    want: TriState,
}
pub struct Page<'a> {
    text: &'a str,
    pub item_infoboxes: Vec<ItemInfobox<'a>>,
    #[allow(dead_code)]    
    stub_node: Intent<parse_wiki_text::Node<'a>>,
    navbox_node: Intent<parse_wiki_text::Node<'a>>,
}
pub struct ItemInfobox<'a> {
    original_fragment: parse_wiki_text::Node<'a>,
    pub name: String,
    pub params: indexmap::IndexMap<String, String>,
}
impl<'a> ItemInfobox<'a> {
    fn format(&self) -> String {
        use itertools::Itertools;
        let out = "{{Item".to_string()
            + &self
                .params
                .iter()
                .map(|(k, v)| format!("\n | {} = {}", k, v))
                .join("")
            + "\n}}";
        out
    }
}
pub fn parse_page(page: &str) -> Page {
    let parsed = parse_wiki_text::Configuration::default().parse(page);
    assert!(parsed.warnings.is_empty());
    let mut infoboxen = vec![];
    let mut stub_node = None;
    let mut navbox_node = None;
    for node in parsed.nodes {
        match &node {
            parse_wiki_text::Node::Template {
                start,
                end,
                name,
                parameters,
            } => {
                assert_eq!(name.len(), 1);
                match name[0] {
                    parse_wiki_text::Node::Text { value, .. } => match value {
                        "Item" => {
                            let params_parsed = parameters
                                .iter()
                                .map(|p| (p.name_str(page).to_owned(), p.val_str(page).to_owned()))
                                .collect::<indexmap::IndexMap<_, _>>();
                            infoboxen.push(ItemInfobox {
                                original_fragment: node,
                                name: params_parsed["title"].to_owned(),
                                params: params_parsed,
                            });
                        }
                        "Stub" => stub_node = Some(node),
                        "Items Navbox" => navbox_node = Some(node),
                        _ => {} //just some text, don't worry about it
                    },
                    _ => unimplemented!("{:?}", name),
                }
            }
            _ => {}
        }
    }
    assert_eq!(infoboxen.len(), 1);
    Page {
        text: page,
        item_infoboxes: infoboxen,
        stub_node: Intent {
            have: stub_node,
            want: TriState::Idk,
        },
        navbox_node: Intent {
            have: navbox_node,
            want: TriState::Yes,
        },
    }
}

pub fn reformat_page<'a>(page: &'a Page) -> String {
    let mut result = page.text.to_string();
    for ibox in page.item_infoboxes.iter() {
        result.replace_range(ibox.original_fragment.range(), &ibox.format());
    }
    match (&page.navbox_node.have, &page.navbox_node.want) {
        (None, TriState::Yes) => result.push_str("\n{{Items Navbox}}"),
        _ => {}
    };
    result
}

pub struct UpdateResult<'a> {
    pub new_text: String,
    pub meaningful_change: bool,
    pub old_version: Option<&'a str>,
}

pub fn update_infobox<'a, S1, S2>(
    page_text: &'a str,
    new_ibox_values: &[(S1, S2)],
) -> anyhow::Result<UpdateResult<'a>>
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

    // if the page exists, but infobox doesn't - something's very wrong!
    // Like, say, it's a redirect..
    // We need to handle those things case-by-case
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

    let mut meaningful_change = false;
    let ibox = &infoboxen[0];
    let mut updated_parts = vec![];
    let mut used_keys = std::collections::HashSet::new();
    let mut version = None;
    for param in &ibox.2 {
        let name = param.name.as_ref().unwrap();
        assert_eq!(name.len(), 1);
        let name = param.name_str(page_text);
        let old_param_str = &page_text[param.start..param.end];
        let old_val = param.val_str(page_text);
        let new_val: Option<&str> = new_ibox_values
            .iter()
            .find(|(n, _)| n.as_ref().to_lowercase() == name.to_lowercase())
            .map(|x| x.1.as_ref().trim());
        if name == "lastUpdate" {
            version = Some(old_val);
        }

        used_keys.insert(name);

        match new_val {
            None => {
                updated_parts.push(old_param_str.to_string());
            }
            Some(new_val) => {
                if old_val != new_val && name != "lastUpdate" {
                    meaningful_change = true;
                    println!("{}:\n{:?}\n  != \n{:?}", name, old_val, new_val);
                }
                updated_parts.push(format!("{} = {}", name, new_val));
            }
        }
    }

    let new_ibox = _make_infobox_skipping_some(new_ibox_values, updated_parts, Some(&used_keys));
    let new_page = page_text[..ibox.0].to_string() + &new_ibox + &page_text[ibox.1..];

    Ok(UpdateResult {
        new_text: new_page,
        meaningful_change,
        old_version: version,
    })
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
