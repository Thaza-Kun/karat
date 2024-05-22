#![allow(unused_variables, unused_imports)]
use clap::builder::Str;
use clap::{Args, Parser, Subcommand};
use colored::{ColoredString, Colorize};
use lazy_static::lazy_static;
// use pulldown_cmark::{CodeBlockKind, CowStr, Event, Parser, Tag};
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{format, write, Display, Formatter};
use std::path::{Path, PathBuf};
use std::string::ParseError;
use std::{default, fs, io::Write, str::FromStr};

lazy_static! {
    static ref DATE_RE: Regex = Regex::new(
        r"(?x)
        (?P<year>\d{4})  # the year
        -
        (?P<month>\d{2}) # the month
        -
        (?P<day>\d{2})   # the day
        ",
    )
    .expect("Regex run error");
}

struct Date(chrono::NaiveDate);

impl Default for Date {
    fn default() -> Self {
        Self(Default::default())
    }
}

struct List(Vec<String>);

struct Count(usize);

impl Into<usize> for Count {
    fn into(self) -> usize {
        self.0
    }
}

impl From<String> for Count {
    fn from(value: String) -> Self {
        Self(value.len())
    }
}

impl Default for Count {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl Default for List {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl From<String> for List {
    fn from(value: String) -> Self {
        let mut data = Vec::new();
        for v in value.split("::") {
            data.push(v.trim().into())
        }
        Self(data)
    }
}

impl From<String> for Date {
    fn from(value: String) -> Self {
        if DATE_RE.captures(&value).is_some() {
            Self(chrono::NaiveDate::from_str(&value).expect(""))
        } else {
            Self(chrono::NaiveDate::from_str("").expect(""))
        }
    }
}

impl Into<chrono::NaiveDate> for Date {
    fn into(self) -> chrono::NaiveDate {
        self.0
    }
}
impl Into<Vec<String>> for List {
    fn into(self) -> Vec<String> {
        self.0
    }
}

trait Extract {
    fn read<T>(&self, key: &str) -> Option<T>
    where
        T: From<String>;
}

impl Extract for HashMap<String, String> {
    fn read<T>(&self, key: &str) -> Option<T>
    where
        T: From<String>,
    {
        match self.get(key) {
            Some(v) => Some(T::from(v.trim().to_string().clone())),
            None => None,
        }
    }
}

#[derive(Clone, Default, Debug)]
struct FileData {
    date: chrono::NaiveDate,
    parents: Vec<String>,
    hashtags: Vec<String>,
    links: usize,
    name: String,
}

impl FileData {
    fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }
}

impl From<HashMap<String, String>> for FileData {
    fn from(value: HashMap<String, String>) -> Self {
        Self {
            date: value.read::<Date>("tarikh").unwrap_or_default().into(),
            parents: value.read::<List>("idx-naik").unwrap_or_default().into(),
            hashtags: value.read::<List>("hashtag").unwrap_or_default().into(),
            links: value.read::<Count>("links").unwrap_or_default().into(),
            ..default::Default::default()
        }
    }
}

impl Display for FileData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -[🔗{:>3}]- {}\n<:{}::{}>",
            self.date.to_string().yellow(),
            self.links,
            self.name,
            self.hashtags
                .iter()
                .map(|i| i.underline().to_string())
                .collect::<Vec<String>>()
                .join("::"),
            self.parents
                .iter()
                .map(|i| i.blue().to_string())
                .collect::<Vec<String>>()
                .join("::"),
        )
    }
}

fn extract_keywords(stream: Vec<&str>) -> HashMap<String, String> {
    let hashtag_rexp: Regex = Regex::new("^#[a-zA-Z0-9/_]+").expect("Regex Error");
    let link_rexp: Regex = Regex::new(r#"\[\[([^\]\]])*\]\]"#).expect("Regex Error");
    let mut metadata = HashMap::<String, String>::new();
    for s in stream {
        if let Some((key, value)) = s.split_once("::") {
            for val in value.split(",") {
                match metadata.get(key) {
                    Some(v) => metadata.insert(key.into(), format!("{} :: {}", v, val.trim())),
                    None => metadata.insert(key.into(), val.trim().into()),
                };
            }
        }
        for token in s.split_whitespace() {
            if hashtag_rexp.captures(token).is_some() {
                match metadata.get("hashtag") {
                    Some(v) => {
                        metadata.insert("hashtag".into(), format!("{} :: {}", v, token.trim()))
                    }
                    None => metadata.insert("hashtag".into(), token.trim().into()),
                };
            }
        }
        for l in link_rexp.captures_iter(s) {
            match metadata.get("links") {
                Some(c) => metadata.insert("links".into(), format!("{}{}", c, "x")),
                None => metadata.insert("links".into(), "x".into()),
            };
        }
    }
    metadata
}

impl Command for FileList {
    fn execute(&self) -> Result<(), std::io::Error> {
        let pattern = Regex::new(&self.with).expect("Regex build error");
        let paths = fs::read_dir(&self.from)?;
        let mut file_vec: Vec<FileData> = Vec::new();
        for path in paths {
            let file = path.unwrap().path();
            if file.is_file() {
                let name = file.file_name().unwrap();
                let content = fs::read_to_string(file.clone())?;

                let stream: Vec<&str> = content.split_terminator(&['\r', '\n'][..]).collect();
                let metadata = extract_keywords(stream);
                // let (key, value) = stream[0].split_once("::").unwrap_or(("", ""));
                // if DATE_RE.captures(value).is_some() {
                file_vec.push(FileData::from(metadata).with_name(name.to_str().unwrap().into()));
                // println!("{:#?}", &file_vec);
                // }
            }
        }
        file_vec.sort_by_key(|x| x.date);
        println!("Read {} files", &file_vec.len());
        let file_vec = file_vec
            .iter()
            .filter(|&i| pattern.captures(&i.name).is_some())
            .collect::<Vec<&FileData>>();
        if self.to.is_some() {
            let mut list = fs::File::create(&self.clone().to.unwrap())?;
            for i in &file_vec {
                list.write_all(format!("{}\n", i).as_bytes()).unwrap();
            }
        } else {
            for f in file_vec
                .iter()
                .enumerate()
                .map(|(n, i)| format!("[{: >2}] {}", n + 1, i))
                .take(self.limit)
            {
                println!("{}", f);
            }
            println!("---");
            println!(
                "Use `cli read {} -n <N> --with \"{}\"` to read content number <N>",
                self.from.as_path().as_os_str().to_str().unwrap(),
                self.with
            )
        }
        Ok(())
    }
}

impl Command for FileRead {
    fn execute(&self) -> Result<(), std::io::Error> {
        let pattern = Regex::new(&self.with).expect("Regex build error");
        let paths = fs::read_dir(&self.from)?;
        let mut file_vec: Vec<FileData> = Vec::new();
        for path in paths {
            let file = path.unwrap().path();
            if file.is_file() {
                let name = file.file_name().unwrap();
                let content = fs::read_to_string(file.clone())?;

                let stream: Vec<&str> = content.split_terminator(&['\r', '\n'][..]).collect();
                let metadata = extract_keywords(stream);
                file_vec.push(FileData::from(metadata).with_name(name.to_str().unwrap().into()))
            }
        }
        file_vec.sort_by_key(|x| x.date);
        let file_vec = file_vec
            .iter()
            .filter(|&i| pattern.captures(&i.name).is_some())
            .collect::<Vec<&FileData>>();
        let re = fs::read_to_string(PathBuf::from_iter(vec![
            self.from.clone(),
            file_vec[self.number.saturating_sub(1)].clone().name.into(),
        ]))?;
        println!("{}", file_vec[self.number.saturating_sub(1)]);
        println!("{}", re);
        Ok(())
    }
}

#[derive(Args, Clone)]
struct FileList {
    from: PathBuf,
    to: Option<PathBuf>,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long, default_value = ".")]
    with: String,
}

#[derive(Args)]
struct FileRead {
    from: PathBuf,
    #[arg(short)]
    number: usize,
    #[arg(long, default_value = ".")]
    with: String,
}

#[derive(Parser)]
enum App {
    List(FileList),
    Read(FileRead),
}

trait Command {
    fn execute(&self) -> Result<(), std::io::Error>;
}

fn main() {
    if let Err(e) = match App::parse() {
        App::List(cmd) => cmd.execute(),
        App::Read(cmd) => cmd.execute(),
    } {
        println!("{:?}", e)
    }
}

// #[derive(Debug)]
// struct SelfError;

// fn parse_content(content: String) -> Result<DocumentAST, SelfError> {
//     let parser = pulldown_cmark::Parser::new(content.as_str());
//     Ok(DocumentAST::from(parser))
// }
// #[derive(Debug)]
// struct DocumentMetadata;

// impl Display for DocumentMetadata {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Metadata")
//     }
// }

// #[derive(Debug)]
// struct DocumentAST {
//     meta: DocumentMetadata,
//     content: Vec<BlockElement>,
// }

// impl Display for DocumentAST {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "DocumentAST [\n\t({}){}\n] DocumenAST",
//             self.meta,
//             self.content
//                 .iter()
//                 .fold("".to_string(), |a, b| format!("{a}\n\t {}", b))
//         )
//     }
// }

// impl From<Parser<'_, '_>> for DocumentAST {
//     fn from(value: Parser<'_, '_>) -> Self {
//         let mut children: Vec<BlockElement> = Vec::new();
//         let mut mut_val = value.map(|x| x).collect::<Vec<Event>>().into_iter();
//         while let Some(event) = mut_val.next() {
//             match event {
//                 Event::End(_) => continue,
//                 Event::Start(tag) => match tag {
//                     Tag::Paragraph => {
//                         let mut grandchild: Vec<Vec<InlineElement>> = Vec::new();
//                         let mut grandgrandchild: Vec<InlineElement> = Vec::new();
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::Paragraph) => break,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                                     grandgrandchild.push(InlineElement::Str(s.to_string()))
//                                 }
//                                 Event::Rule => {
//                                     grandgrandchild.push(InlineElement::Str("---".to_string()))
//                                 }
//                                 Event::SoftBreak => grandgrandchild.push(InlineElement::SoftBreak),
//                                 any => unimplemented!("Inside Tag::Paragraph -> {:?}", any),
//                             }
//                         }
//                         grandchild.push(grandgrandchild);
//                         children.push(BlockElement::LineBlock(grandchild).concat_string())
//                     }
//                     Tag::Heading(level, iden, classes) => {
//                         let l: u8 = level as u8;
//                         let ident = iden.unwrap_or_default().to_string();
//                         if let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::Heading(_, _, _)) => continue,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                                     children.push(BlockElement::Header(
//                                         l,
//                                         HTMLItem {
//                                             ident,
//                                             classes: classes
//                                                 .iter()
//                                                 .map(|x| x.to_owned().to_string())
//                                                 .collect(),
//                                             attrs: Vec::new(),
//                                             children: InlineVec { items: vec![InlineElement::Str(s.to_string())] },
//                                         },
//                                     ))
//                                 }
//                                 any => unimplemented!("Inside Tag::Heading -> {:?}", any),
//                             }
//                         }
//                     }
//                     Tag::BlockQuote => {
//                         let mut grandchild: Vec<BlockElement> = Vec::new();
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::BlockQuote) => break,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => grandchild
//                                     .push(BlockElement::Plain(vec![InlineElement::Str(
//                                         s.to_string(),
//                                     )])),
//                                 _ => continue,
//                             }
//                         }
//                         children.push(BlockElement::BlockQuote(grandchild));
//                     }
//                     Tag::Emphasis => {
//                         let mut grandchild: Vec<InlineElement> = Vec::new();
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::BlockQuote) => break,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                                     grandchild.push(InlineElement::Str(s.to_string()))
//                                 }
//                                 _ => continue,
//                             }
//                         }
//                         children.push(BlockElement::Plain(vec![InlineElement::Emph(grandchild)]));
//                     }
//                     Tag::Strong => {
//                         let mut grandchild: Vec<InlineElement> = Vec::new();
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::BlockQuote) => break,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                                     grandchild.push(InlineElement::Str(s.to_string()))
//                                 }
//                                 _ => continue,
//                             }
//                         }
//                         children.push(BlockElement::Plain(vec![InlineElement::Strong(grandchild)]));
//                     }
//                     Tag::Strikethrough => {
//                         let mut grandchild: Vec<InlineElement> = Vec::new();
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::BlockQuote) => break,
//                                 Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                                     grandchild.push(InlineElement::Str(s.to_string()))
//                                 }
//                                 _ => continue,
//                             }
//                         }
//                         children.push(BlockElement::Plain(vec![InlineElement::Strikeout(
//                             grandchild,
//                         )]));
//                     }
//                     Tag::List(num) => {
//                         let mut grandgrandchild: Vec<Vec<BlockElement>> = Vec::new();
//                         let mut grandgrandgrandchild: Vec<InlineElement> = Vec::new();
//                         // TODO: Unnumberred list
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::List(_)) => break,
//                                 Event::Start(Tag::Item) => {
//                                     while let Some(next) = mut_val.next() {
//                                         match next {
//                                                 Event::End(Tag::Item) => break,
//                                                 Event::Text(s) => grandgrandgrandchild.push(InlineElement::Str(s.to_string())),
//                                                 Event::Code(s) => grandgrandgrandchild.push(InlineElement::Code(HTMLItem{children: s.to_string(), ..Default::default()})),
//                                                 Event::SoftBreak => grandgrandgrandchild.push(InlineElement::SoftBreak),
//                                                 Event::Start(Tag::List(_)) => {
//                                                     while let Some(next_) = mut_val.next() {
//                                                         match next_ {
//                                                             Event::End(Tag::List(_)) => break,
//                                                             _ => continue
//                                                         }
//                                                     }
//                                                     // todo!("Implement nested list Event::Start(Tag::List(Some(u64))) > Event::Start(Tag::Item) > Event::Start(Tag::List(Option<u64>))")
//                                                 }
//                                                 any => todo!("Event::Start(Tag::List(Some(u64))) > Event::Start(Tag::Item) > {:?}", any)
//                                             }
//                                     }
//                                 }
//                                 any => {
//                                     unimplemented!("Event::Start(Tag::List(Some(u68)) > {:?}", any)
//                                 }
//                             }
//                         }
//                         grandgrandchild.push(vec![BlockElement::Plain(grandgrandgrandchild)]);
//                         children.push(match num {
//                             Some(i) => BlockElement::OrderedList(
//                                 (
//                                     num.unwrap_or(0) as u8,
//                                     "decimal".to_string(),
//                                     ".".to_string(),
//                                 ),
//                                 grandgrandchild,
//                             ),
//                             None => BlockElement::BulletList(grandgrandchild),
//                         })
//                     }
//                     Tag::CodeBlock(kind) => {
//                         // TODO
//                         match kind {
//                             CodeBlockKind::Indented => {}
//                             CodeBlockKind::Fenced(_) => {}
//                         }
//                         while let Some(next) = mut_val.next() {
//                             match next {
//                                 Event::End(Tag::CodeBlock(_)) => break,
//                                 _ => continue,
//                             }
//                         }
//                     }
//                     any => todo!("Event::Start({:?})", any),
//                 },
//                 Event::Rule => children.push(BlockElement::HorizontalRule),
//                 Event::Text(s) => match s {
//                     CowStr::Boxed(_) => todo!("Cowstr::Boxed(_)"),
//                     CowStr::Borrowed(x) => {
//                         children.push(BlockElement::Plain(vec![InlineElement::Str(x.to_string())]))
//                     }
//                     CowStr::Inlined(_) => todo!("Cowstr::Inlined(_)"),
//                 },
//                 Event::Code(s) => {
//                     children.push(BlockElement::Plain(vec![InlineElement::Code(HTMLItem {
//                         children: s.to_string(),
//                         ..Default::default()
//                     })]))
//                 }
//                 Event::SoftBreak => {
//                     children.push(BlockElement::Plain(vec![InlineElement::SoftBreak]))
//                 }
//                 a => todo!("{:?}", a),
//             }
//         }
//         Self {
//             content: children,
//             meta: DocumentMetadata,
//         }
//     }
// }

// #[derive(Debug)]
// enum MathItem {
//     Display(String),
//     Inline(String),
// }

// impl Display for MathItem {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "Math {}",
//             match self {
//                 MathItem::Display(s) => "Display {s}".to_string(),
//                 MathItem::Inline(s) => "Inline {s}".to_string(),
//             }
//         )
//     }
// }

// #[derive(Debug)]
// struct LinkItem {
//     ident: String,
//     classes: Vec<String>,
//     attrs: Vec<(String, String)>,
//     children: Vec<InlineElement>,
//     /// (URL, Text)
//     target: (String, String),
// }

// impl Display for LinkItem {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "Link\n({}, [{}], [{}])",
//             self.ident,
//             self.classes
//                 .iter()
//                 .fold("".to_string(), |a, b| format!("{a}, \"{b}\"")),
//             self.attrs
//                 .iter()
//                 .fold("".to_string(), |a, (k, v)| format!("{a}, (\"{k}\", \"{v}\")"))
//         )
//     }
// }

// /// The struct resembling HTML tags, i.e. `<div>children</div>`
// ///
// /// Pandoc Native representation:
// /// ```haskell
// /// Elem
// ///     (iden, classes, attrs)
// ///     [ children ]
// /// ```
// #[derive(Debug, Default)]
// struct HTMLItem<Child> {
//     ident: String,
//     classes: Vec<String>,
//     attrs: Vec<(String, String)>,
//     children: Child,
// }

// #[derive(Debug)]
// struct BlockVec {
//     items: Vec<BlockElement>,
// }

// impl Display for BlockVec {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             self.items
//                 .iter()
//                 .fold("".to_string(), |a, b| format!("{a} {b}"))
//         )
//     }
// }
// #[derive(Debug)]
// struct InlineVec {
//     items: Vec<InlineElement>,
// }

// impl Display for InlineVec {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             self.items
//                 .iter()
//                 .fold("".to_string(), |a, b| format!("{a} {b}"))
//         )
//     }
// }
// #[derive(Debug)]
// struct StringVec {
//     items: Vec<String>,
// }

// impl Display for StringVec {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "[{}]",
//             self.items
//                 .iter()
//                 .fold("".to_string(), |a, b| format!("{a} \"{b}\""))
//         )
//     }
// }

// impl<Child> Display for HTMLItem<Child>
// where
//     Child: Display,
// {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "\n\t\t{}",
//             format!(
//                 "({}, {}, {})\n\t\t{}",
//                 self.ident,
//                 self.classes
//                     .iter()
//                     .fold("".to_string(), |a, b| format!("{a}, \"{b}\"")),
//                 self.attrs
//                     .iter()
//                     .fold("".to_string(), |a, (k, v)| format!("{a}, (\"{k}\", \"{v}\")")),
//                 self.children
//             )
//         )
//     }
// }

// // TODO
// #[derive(Debug)]
// struct TableTag;

// #[derive(Debug)]
// enum QuoteType {
//     SingleQuote,
//     DoubleQuote,
// }

// // TODO
// #[derive(Debug)]
// struct Citation;

// /// Elements are defined from [pandoc](https://hackage.haskell.org/package/pandoc-types-1.22/docs/Text-Pandoc-Definition.html)
// #[derive(Debug)]
// enum Element {
//     Inline(InlineElement),
//     Block(BlockElement),
// }

// impl From<Event<'_>> for Element {
//     fn from(value: Event<'_>) -> Self {
//         match value {
//             Event::Rule => Self::Inline(InlineElement::SoftBreak),
//             Event::Text(s) | Event::Code(s) | Event::Html(s) => {
//                 Self::Inline(InlineElement::Str(s.to_string()))
//             }
//             _ => Self::Block(BlockElement::Null),
//         }
//     }
// }

// #[derive(Debug)]
// enum InlineElement {
//     Str(String),
//     Emph(Vec<InlineElement>),
//     Underline(Vec<InlineElement>),
//     Strong(Vec<InlineElement>),
//     Strikeout(Vec<InlineElement>),
//     Superscript(Vec<InlineElement>),
//     Subscript(Vec<InlineElement>),
//     SmallCaps(Vec<InlineElement>),
//     Quoted(QuoteType, Vec<Element>),
//     Cite(Vec<Citation>, Vec<InlineElement>),
//     Code(HTMLItem<String>),
//     Space,
//     SoftBreak,
//     LineBreak,
//     Math(MathItem),
//     /// (Format, Text)
//     RawInline(String, String),
//     Link(LinkItem),
//     Image(LinkItem),
//     Note(BlockElement),
//     Span(HTMLItem<InlineVec>),
// }

// impl Display for InlineElement {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             match self {
//                 InlineElement::Str(s) => format!("Str {s}"),
//                 InlineElement::Emph(s) => format!("Emph [{:?}]", s),
//                 InlineElement::Underline(s) => format!("Underline [{:?}]", s),
//                 InlineElement::Strong(s) => format!("Strong [{:?}]", s),
//                 InlineElement::Strikeout(s) => format!("Strikeout [{:?}]", s),
//                 InlineElement::Superscript(s) => format!("Superscript [{:?}]", s),
//                 InlineElement::Subscript(s) => format!("Subscript [{:?}]", s),
//                 InlineElement::SmallCaps(s) => format!("SmallCaps [{:?}]", s),
//                 InlineElement::Quoted(t, s) => format!("Quoted ({:?} [{:?}])", t, s),
//                 InlineElement::Cite(c, s) => format!("Cite ([{:?}] [{:?}])", c, s),
//                 InlineElement::Code(s) => format!("Code {s}"),
//                 InlineElement::Space => format!("Space"),
//                 InlineElement::SoftBreak => format!("SoftBreak"),
//                 InlineElement::LineBreak => format!("LineBreak"),
//                 InlineElement::Math(s) => format!("Math {s}"),
//                 InlineElement::RawInline(f, s) => format!("RawInLine {f} {s}"),
//                 InlineElement::Link(s) => format!("Link {s}"),
//                 InlineElement::Image(s) => format!("Image {s}"),
//                 InlineElement::Note(s) => format!("Note {s}"),
//                 InlineElement::Span(s) => format!("Span {:#?}", s),
//             }
//         )
//     }
// }

// impl InlineElement {
//     pub fn to_string(&self) -> String {
//         match self {
//             InlineElement::Str(s) => s.clone(),
//             InlineElement::SoftBreak => "\n".to_string(),
//             any => todo!("InlineElement.to_string() for {:?}", any),
//         }
//     }
// }
// #[derive(Debug)]
// enum BlockElement {
//     Plain(Vec<InlineElement>),
//     Para(Vec<InlineElement>),
//     LineBlock(Vec<Vec<InlineElement>>),
//     CodeBlock(HTMLItem<String>),
//     /// (Format, Text)
//     RawBlock(String, String),
//     BlockQuote(Vec<BlockElement>),
//     /// ((numbering, style, delim), \[Block\])
//     OrderedList((u8, String, String), Vec<Vec<BlockElement>>),
//     BulletList(Vec<Vec<BlockElement>>),
//     DefinitionList(Vec<(Vec<InlineElement>, Vec<Vec<BlockElement>>)>),
//     Header(u8, HTMLItem<InlineVec>),
//     HorizontalRule,
//     Table(TableTag),
//     Div(HTMLItem<BlockVec>),
//     Null,
// }

// impl Display for BlockElement {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "{}",
//             match self {
//                 BlockElement::Plain(s) => s
//                     .iter()
//                     .fold("Plain ".to_string(), |a, b| format!("{a} {b}")),
//                 BlockElement::Para(s) => s
//                     .iter()
//                     .fold("Para ".to_string(), |a, b| format!("{a} {b}")),
//                 BlockElement::LineBlock(s) =>
//                     s.iter().fold("LineBlock".to_string(), |a, b| format!(
//                         "{a} {}",
//                         b.iter().fold("".to_string(), |x, y| format!("{x} {y}")),
//                     )),
//                 BlockElement::CodeBlock(s) => "CodeBlock {s}".to_string(),
//                 BlockElement::RawBlock(f, s) => "RawBlock {f} {s}".to_string(),
//                 BlockElement::BlockQuote(s) => s
//                     .iter()
//                     .fold("BlockQuote".to_string(), |a, b| format!("{a} {b}")),
//                 BlockElement::OrderedList((i, f, d), s) => format!(
//                     "OrderedList ({i}, {f}, {d}) {}",
//                     s.iter().fold("".to_string(), |a, b| format!(
//                         "{a} {}",
//                         b.iter().fold("".to_string(), |x, y| format!("{x} {y}"))
//                     ))
//                 ),
//                 BlockElement::BulletList(s) =>
//                     s.iter().fold("LineBlock".to_string(), |a, b| format!(
//                         "{a} {}",
//                         b.iter().fold("".to_string(), |x, y| format!("{x} {y}")),
//                     )),
//                 BlockElement::DefinitionList(s) => format!("DefinitionList {:?}", s),
//                 BlockElement::Header(i, s) => format!("Header {i} {s}"),
//                 BlockElement::HorizontalRule => "HorizontalRule".to_string(),
//                 BlockElement::Table(s) => format!("{:#?}", s),
//                 BlockElement::Div(s) => format!("Div {s}"),
//                 BlockElement::Null => "Null".to_string(),
//             }
//         )
//     }
// }

// impl BlockElement {
//     pub fn concat_string(self) -> Self {
//         match self {
//             // TODO Handle case where other element is between two str element
//             BlockElement::Plain(s) => {
//                 let v = s.iter().fold("".to_string(), |a, b| {
//                     format!("{a}{}", b.to_string().as_str())
//                 });
//                 BlockElement::Plain(vec![InlineElement::Str(v)])
//             }
//             BlockElement::Para(s) => {
//                 let v = s.iter().fold("".to_string(), |a, b| {
//                     format!("{a}{}", b.to_string().as_str())
//                 });
//                 BlockElement::Para(vec![InlineElement::Str(v)])
//             }
//             BlockElement::LineBlock(ss) => {
//                 let mut new: Vec<InlineElement> = Vec::new();
//                 for s in ss {
//                     let v = s.iter().fold("".to_string(), |a, b| {
//                         format!("{a}{}", b.to_string().as_str())
//                     });
//                     new.push(InlineElement::Str(v));
//                 }
//                 BlockElement::LineBlock(vec![new])
//             }
//             any => todo!("BlockElement::concat_string() for {:?}", any),
//         }
//     }
// }
