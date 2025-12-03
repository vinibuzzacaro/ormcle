use std::str::FromStr;

use proc_macro::{Delimiter, Group, TokenStream, TokenTree, token_stream::IntoIter};

macro_rules! push_field {
    ($field_name: expr, $column_name: expr, $str: expr) => {{
        $str.push_str(&$field_name);
        $str.push_str(": row.get(\"");
        $str.push_str(&$column_name);
        $str.push_str("\")?,");
    }};
}

#[derive(Debug)]
enum OrmcleError {
    InvalidKey,
    InvalidFlag,
}

#[derive(Debug)]
enum Key {
    TableName,
    ColumnName,
}

#[derive(Debug)]
enum Flag {
    PrimaryKey,
}

#[derive(Debug)]
struct KeyValue {
    key: Key,
    value: String,
}

#[derive(Debug)]
enum Attr {
    Flag(Flag),
    KeyValue(KeyValue),
}

struct AttrIterator {
    ts: IntoIter,
}

impl FromStr for Key {
    type Err = OrmcleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "table_name" => Ok(Self::TableName),
            "column_name" => Ok(Self::ColumnName),
            _ => Err(OrmcleError::InvalidKey),
        }
    }
}

impl FromStr for Flag {
    type Err = OrmcleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "primary_key" => Ok(Self::PrimaryKey),
            _ => Err(OrmcleError::InvalidFlag),
        }
    }
}

impl From<Group> for AttrIterator {
    fn from(value: Group) -> Self {
        Self {
            ts: value.stream().into_iter(),
        }
    }
}

impl Iterator for AttrIterator {
    type Item = Result<Attr, OrmcleError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.ts.next() {
                Some(TokenTree::Ident(i)) => {
                    let key = i.to_string();
                    let maybe_value = self
                        .ts
                        .next()
                        .and_then(|tt| match tt {
                            TokenTree::Punct(p) if p.as_char().eq(&'=') => self.ts.next(),
                            _ => None,
                        })
                        .and_then(|tt| match tt {
                            TokenTree::Literal(l) => Some(l.to_string().replace("\"", "")),
                            _ => None,
                        });
                    let attr = match maybe_value {
                        Some(value) => {
                            let key = match key.parse() {
                                Ok(k) => k,
                                Err(e) => break Some(Err(e)),
                            };
                            Attr::KeyValue(KeyValue { key, value })
                        }
                        None => {
                            let flag = match key.parse() {
                                Ok(f) => f,
                                Err(e) => break Some(Err(e)),
                            };
                            Attr::Flag(flag)
                        }
                    };
                    break Some(Ok(attr));
                }
                Some(_) => continue,
                None => break None,
            }
        }
    }
}

#[derive(Debug)]
enum Attribute {
    Struct {
        table_name: Option<String>,
    },
    Field {
        column_name: Option<String>,
        primary_key: bool,
    },
}

#[derive(Debug)]
struct Entity {
    struct_name: String,
    attr: Option<Attribute>,
}

#[derive(Debug)]
struct Field {
    field_name: String,
    field_type: String,
    attr: Option<Attribute>,
}

struct FieldIterator {
    ts: IntoIter,
}

impl From<Group> for FieldIterator {
    fn from(value: Group) -> Self {
        Self {
            ts: value.stream().into_iter(),
        }
    }
}

impl Iterator for FieldIterator {
    type Item = Field;

    fn next(&mut self) -> Option<Self::Item> {
        let mut attr = Attribute::default_field();
        let mut some_element = false;
        loop {
            match self.ts.next() {
                Some(TokenTree::Punct(p)) if p.as_char().eq(&'#') => {
                    let mut group = match self.ts.next() {
                        Some(TokenTree::Group(g)) => g.stream().into_iter(),
                        _ => unreachable!(),
                    };
                    let maybe_iter = group
                        .next()
                        .filter(|tt| matches!(tt, TokenTree::Ident(i) if i.to_string() == "ormcle"))
                        .and_then(|_| group.next())
                        .and_then(|tt| match tt {
                            TokenTree::Group(g) => Some(g),
                            _ => None,
                        })
                        .map(|g| AttrIterator::from(g));
                    if let Some(mut attr_iter) = maybe_iter {
                        while let Some(attribute) = attr_iter.next() {
                            some_element = true;
                            match attribute {
                                Ok(Attr::Flag(Flag::PrimaryKey)) => attr = attr.with_primary_key(),
                                Ok(Attr::KeyValue(KeyValue {
                                    key: Key::ColumnName,
                                    value,
                                })) => attr = attr.with_column_name(value),
                                Ok(_) => unreachable!(),
                                Err(e) => panic!("{e:?}"),
                            }
                        }
                    }
                }
                Some(TokenTree::Ident(i)) => {
                    let field_name = i.to_string();
                    let colon_found = self
                        .ts
                        .next()
                        .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&':')))
                        .unwrap_or(false);
                    if !colon_found {
                        return None;
                    }
                    let field_type = match self.ts.next() {
                        Some(TokenTree::Ident(i)) => {
                            let mut ty = i.to_string();
                            let opening_bracket = self
                                .ts
                                .next()
                                .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&'<')))
                                .unwrap_or(false);
                            if !opening_bracket {
                                ty
                            } else {
                                ty.push('<');
                                loop {
                                    match self.ts.next() {
                                        Some(TokenTree::Punct(p)) => {
                                            let p_as_char = p.as_char();
                                            let type_end = p_as_char.eq(&'>');
                                            ty.push(p_as_char);
                                            if type_end {
                                                break;
                                            }
                                        }
                                        Some(TokenTree::Ident(i)) => {
                                            ty.push_str(i.to_string().as_str());
                                        }
                                        None => return None,
                                        _ => continue,
                                    }
                                }
                                ty
                            }
                        }
                        _ => continue,
                    };
                    let attr = some_element.then_some(attr);
                    return Some(Self::Item {
                        field_name,
                        field_type,
                        attr,
                    });
                }
                Some(_) => continue,
                None => return None,
            }
        }
    }
}

impl Field {
    fn is_pk(&self) -> bool {
        self.attr.as_ref().map_or(false, |attr| match attr {
            Attribute::Struct { .. } => unreachable!(),
            Attribute::Field { primary_key, .. } => *primary_key,
        })
    }

    fn try_from_iter(ts: &mut impl Iterator<Item = TokenTree>) -> Option<Self> {
        let mut attr = None;
        while let Some(tt) = ts.next() {
            match tt {
                TokenTree::Punct(p) if p.as_char().eq(&'#') => {
                    let mut group = match ts.next() {
                        Some(TokenTree::Group(g)) => g.stream().into_iter(),
                        _ => unreachable!(),
                    };
                    let maybe_group = group
                        .next()
                        .filter(|tt| matches!(tt, TokenTree::Ident(i) if i.to_string() == "ormcle"))
                        .and_then(|_| group.next())
                        .and_then(|tt| match tt {
                            TokenTree::Group(g) => Some(g),
                            _ => None,
                        });
                    if let Some(attr_group) = maybe_group {
                        attr = Attribute::try_parse_field(attr_group);
                    }
                }
                TokenTree::Ident(i) => {
                    let field_name = i.to_string();
                    let colon_found = ts
                        .next()
                        .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&':')))
                        .unwrap_or(false);
                    if !colon_found {
                        return None;
                    }
                    let field_type = match ts.next() {
                        Some(TokenTree::Ident(i)) => {
                            let mut ty = i.to_string();
                            let opening_bracket = ts
                                .next()
                                .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&'<')))
                                .unwrap_or(false);
                            if !opening_bracket {
                                ty
                            } else {
                                ty.push('<');
                                loop {
                                    match ts.next() {
                                        Some(TokenTree::Punct(p)) => {
                                            let p_as_char = p.as_char();
                                            let type_end = p_as_char.eq(&'>');
                                            ty.push(p_as_char);
                                            if type_end {
                                                break;
                                            }
                                        }
                                        Some(TokenTree::Ident(i)) => {
                                            ty.push_str(i.to_string().as_str());
                                        }
                                        None => return None,
                                        _ => unreachable!(),
                                    }
                                }
                                ty
                            }
                        }
                        _ => unreachable!(),
                    };
                    if attr.is_some() {
                        return None;
                    }
                    return Some(Self {
                        field_name,
                        field_type,
                        attr,
                    });
                }
                _ => continue,
            }
        }
        None
    }
}

impl Attribute {
    fn default_struct() -> Self {
        Self::Struct { table_name: None }
    }

    fn default_field() -> Self {
        Self::Field {
            column_name: None,
            primary_key: false,
        }
    }

    fn with_table_name(self, value: String) -> Self {
        match self {
            Attribute::Struct { table_name: _ } => Attribute::Struct {
                table_name: Some(value),
            },
            Attribute::Field { .. } => self,
        }
    }

    fn with_column_name(self, value: String) -> Self {
        match self {
            Attribute::Struct { .. } => self,
            Attribute::Field { primary_key, .. } => Attribute::Field {
                column_name: Some(value),
                primary_key,
            },
        }
    }

    fn with_primary_key(self) -> Self {
        match self {
            Attribute::Struct { .. } => self,
            Attribute::Field { column_name, .. } => Attribute::Field {
                column_name,
                primary_key: true,
            },
        }
    }

    fn try_parse_struct(group: Group) -> Option<Self> {
        let mut ts = group.stream().into_iter();
        let mut attr = Attribute::default_struct();
        while let Some(tt) = ts.next() {
            match tt {
                TokenTree::Ident(i) => {
                    let maybe_field = i.to_string();
                    let equal_found = ts
                        .next()
                        .map(|tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&'=')))
                        .unwrap_or(false);
                    if !equal_found {
                        return None;
                    }
                    let maybe_value = match ts.next() {
                        Some(TokenTree::Literal(l)) => l.to_string().replace("\"", ""),
                        _ => return None,
                    };
                    attr = match maybe_field.as_ref() {
                        "table_name" => attr.with_table_name(maybe_value),
                        _ => return None,
                    };
                }
                _ => continue,
            }
        }
        Some(attr)
    }

    fn try_parse_field(group: Group) -> Option<Self> {
        let mut ts = group.stream().into_iter();
        let mut attr = Attribute::default_field();
        let mut some_element = false;
        while let Some(tt) = ts.next() {
            some_element = true;
            match tt {
                TokenTree::Ident(i) => {
                    let key = i.to_string();
                    match key.as_str() {
                        "primary_key" => attr = attr.with_primary_key(),
                        _ => {
                            let equal_found = ts.next().map_or(
                                false,
                                |tt| matches!(tt, TokenTree::Punct(p) if p.as_char().eq(&'=')),
                            );
                            if !equal_found {
                                return None;
                            }
                            let value = match ts.next() {
                                Some(TokenTree::Literal(l)) => l.to_string().replace("\"", ""),
                                _ => unreachable!(),
                            };
                            match key.as_str() {
                                "column_name" => attr = attr.with_column_name(value.to_uppercase()),
                                _ => continue,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
        some_element.then_some(attr)
    }
}

impl Entity {
    fn try_from_iter(ts: &mut impl Iterator<Item = TokenTree>) -> Option<Self> {
        let mut attr = None;
        while let Some(tt) = ts.next() {
            match tt {
                TokenTree::Ident(i) if i.to_string().eq("struct") => match ts.next() {
                    Some(TokenTree::Ident(i)) => {
                        return Some(Self {
                            struct_name: i.to_string(),
                            attr,
                        });
                    }
                    _ => unreachable!(),
                },
                TokenTree::Punct(p) if p.as_char().eq(&'#') => {
                    let mut group = match ts.next() {
                        Some(TokenTree::Group(g)) => g.stream().into_iter(),
                        _ => unreachable!(),
                    };
                    let ormcle_found = group
                        .next()
                        .map(|tt| matches!(tt, TokenTree::Ident(i) if i.to_string().eq("ormcle")))
                        .unwrap_or(false);
                    if !ormcle_found {
                        continue;
                    }
                    match group.next() {
                        Some(TokenTree::Group(g)) => attr = Attribute::try_parse_struct(g),
                        _ => continue,
                    }
                }
                _ => continue,
            }
        }
        None
    }
}

#[proc_macro_derive(Test)]
pub fn derive_test(input: TokenStream) -> TokenStream {
    let mut ts = input.into_iter();
    let struct_name = loop {
        match ts.next() {
            Some(TokenTree::Ident(i)) if i.to_string().eq("struct") => match ts.next() {
                Some(TokenTree::Ident(i)) => break i.to_string(),
                _ => panic!("Expected struct name"),
            },
            None => panic!("Expected struct definition"),
            _ => continue,
        }
    };
    let group = loop {
        match ts.next() {
            Some(TokenTree::Group(g)) if g.delimiter().eq(&Delimiter::Brace) => {
                break g;
            }
            None => panic!("Expected fields definition"),
            _ => continue,
        }
    };
    let mut field_iter = FieldIterator::from(group);
    let mut output = format!(
        "impl {struct_name} {{
            fn get_row(row: sibyl::Row<'_>) -> sibyl::Result<Self> {{
                Ok(Self {{"
    );
    while let Some(field) = field_iter.next() {
        match field.attr {
            Some(attr) => push_field!(
                field.field_name,
                match attr {
                    Attribute::Struct { .. } => unreachable!(),
                    Attribute::Field { column_name, .. } => column_name.unwrap_or(field.field_name),
                },
                output
            ),
            None => push_field!(field.field_name, field.field_name, output),
        }
    }
    output.push_str("})}}");
    dbg!(&output);
    output.parse().unwrap()
}

#[proc_macro_derive(Id)]
pub fn derive_id(input: TokenStream) -> TokenStream {
    let mut ts = input.into_iter();
    let struct_name = loop {
        match ts.next() {
            Some(TokenTree::Ident(i)) if i.to_string().eq("struct") => match ts.next() {
                Some(TokenTree::Ident(i)) => break i.to_string(),
                _ => panic!("Expected struct name"),
            },
            None => panic!("Expected struct definition"),
            _ => continue,
        }
    };
    let mut inner_ts = loop {
        match ts.next() {
            Some(TokenTree::Group(g)) if g.delimiter().eq(&proc_macro::Delimiter::Brace) => {
                break g.stream().into_iter();
            }
            None => continue,
            _ => panic!("Expected fields definition"),
        }
    };
    let mut fields: Vec<(String, String)> = vec![];
    while let Some(tt) = inner_ts.next() {
        match tt {
            TokenTree::Ident(i) => {
                let field = i.to_string();
                let field_type = inner_ts
                    .next()
                    .and_then(|tt| match tt {
                        TokenTree::Punct(p) if p.as_char().eq(&':') => inner_ts.next(),
                        _ => None,
                    })
                    .and_then(|tt| match tt {
                        TokenTree::Ident(i) => Some(i.to_string()),
                        _ => None,
                    });
                match field_type {
                    Some(ft) => fields.push((field, ft)),
                    None => panic!("Expected field type"),
                }
            }
            _ => continue,
        }
    }
    if fields.is_empty() {
        panic!("Expected fields");
    }
    "const s: &'static str = \"oi\";".parse().unwrap()
}

#[proc_macro_derive(Table, attributes(ormcle))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let mut ts = input.into_iter();
    let entity = Entity::try_from_iter(&mut ts).unwrap();
    let table_name = entity
        .attr
        .and_then(|attr| match attr {
            Attribute::Struct { table_name } => table_name,
            Attribute::Field { .. } => None,
        })
        .unwrap_or(entity.struct_name.clone());
    let struct_name = entity.struct_name;
    let mut fields = String::new();
    let group = loop {
        match ts.next() {
            Some(TokenTree::Group(g)) => break g,
            None => unreachable!(),
            _ => continue,
        }
    };
    let mut id: Option<Field> = None;
    let mut field_iter = FieldIterator::from(group);
    while let Some(field) = field_iter.next() {
        dbg!(&field);
        let column_name = field
            .attr
            .as_ref()
            .and_then(|attr| match attr {
                Attribute::Struct { .. } => unreachable!(),
                Attribute::Field { column_name, .. } => column_name.clone(),
            })
            .unwrap_or(field.field_name.to_uppercase());
        push_field!(field.field_name, column_name, fields);
        match (&id, field.is_pk()) {
            (None, true) => id = Some(field),
            (Some(_), true) => panic!("Only one field can be flagged as ID"),
            _ => (),
        }
    }
    let id_dependent_code = match id {
        Some(i) => {
            let column_name = i
                .attr
                .as_ref()
                .and_then(|attr| match attr {
                    Attribute::Struct { .. } => unreachable!(),
                    Attribute::Field { column_name, .. } => column_name.clone(),
                })
                .unwrap_or(i.field_name.to_uppercase());
            format!("
            #[cfg(feature = \"nonblocking\")]
            pub async fn find_by_id(&'a self, id: {typ}) -> ::sibyl::Result<Option<{struct}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table} WHERE {name} = :ID\").await?;
                let rows = stmt.query((\"ID\", id)).await?;
                if let Some(row) = rows.next().await? {{
                    Ok(Some({struct}  {{ {fields} }}))
                }} else {{
                    Ok(None)
                }}
            }}

            #[cfg(feature = \"blocking\")]
            pub fn find_by_id(&'a self, id: {typ}) -> ::sibyl::Result<Option<{struct}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table} WHERE {name} = :ID\")?;
                let rows = stmt.query((\"ID\", id))?;
                if let Some(row) = rows.next()? {{
                    Ok(Some({struct}  {{ {fields} }}))
                }} else {{
                    Ok(None)
                }}
            }}",
                typ = i.field_type,
                struct = struct_name,
                table = table_name,
                name = column_name
            )
        }
        None => String::new(),
    };
    let output = format!(
        "
            pub struct {struct_name}Repository<'a> {{
                session: sibyl::Session<'a>
            }}

            impl<'a> {struct_name}Repository<'a> {{
               #[cfg(feature = \"nonblocking\")]
                pub async fn find_all(&'a self) -> ::sibyl::Result<Vec<{struct_name}>> {{
                    let stmt = self.session.prepare(\"SELECT * FROM {table_name}\").await?;
                    let rows = stmt.query(()).await?;
                    let mut result = vec![];
                    while let Some(row) = rows.next().await? {{
                        result.push(
                            {struct_name} {{ {fields} }}
                        );
                    }}
                    Ok(result)
                }}

                #[cfg(feature = \"blocking\")]
                pub fn find_all(&'a self) -> ::sibyl::Result<Vec<{struct_name}>> {{
                    let stmt = self.session.prepare(\"SELECT * FROM {table_name}\")?;
                    let rows = stmt.query(())?;
                    let mut result = vec![];
                    while let Some(row) = rows.next()? {{
                        result.push(
                            {struct_name} {{ {fields} }}
                        );
                    }}
                    Ok(result)
                }}

                {id_dependent_code }
            }}
        "
    );
    output.parse().unwrap()
}
