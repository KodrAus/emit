#[derive(Clone)]
pub struct Template<'a>(&'a [Part<'a>]);

impl<'a> Template<'a> {
    pub fn new(parts: &'static [Part<'static>]) -> Template<'a> {
        Template(parts)
    }

    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        todo!()
    }
}

#[derive(Clone)]
pub struct Part<'a>(PartKind<'a>);

impl<'a> Part<'a> {
    pub fn text(text: &'static str) -> Part<'a> {
        Part(PartKind::Text(text))
    }

    pub fn hole(label: &'static str) -> Part<'a> {
        Part(PartKind::Hole(label))
    }
}

#[derive(Clone)]
enum PartKind<'a> {
    Text(&'a str),
    Hole(&'a str),
}
