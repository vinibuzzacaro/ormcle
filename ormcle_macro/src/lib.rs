use proc_macro::{TokenStream, TokenTree};

macro_rules! goto_next_punct {
    ($punct: expr, $tokens: expr) => {
        loop {
            match $tokens.next() {
                Some(TokenTree::Punct(punct)) if punct.as_char().eq(&$punct) => break,
                None => break,
                _ => continue,
            }
        }
    };
}

macro_rules! append_col_getter {
    ($field_name: expr, $fields: expr) => {
        $fields.push_str(&$field_name);
        $fields.push_str(": row.get(\"");
        $fields.push_str(&$field_name.to_uppercase());
        $fields.push_str("\")?,");
    };
}

fn compile_error(msg: &str) -> TokenStream {
    let mut result = String::with_capacity(18 + msg.len());
    result.push_str("compile_error!(\"");
    result.push_str(msg);
    result.push_str("\")");
    result.parse().unwrap()
}

#[proc_macro_derive(Table, attributes(id))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let len = input.to_string().len();
    let mut tokens = input.into_iter();
    let mut table_name = None;
    while let Some(token) = tokens.next() {
        if let TokenTree::Ident(ident) = token {
            let token = ident
                .to_string()
                .eq("struct")
                .then(|| tokens.next())
                .flatten();
            if let Some(TokenTree::Ident(ident)) = token {
                table_name = Some(ident.to_string());
                break;
            }
        }
    }
    let Some(table_name) = table_name else {
        return compile_error("Expected a struct definition");
    };
    let Some(mut tokens) = tokens.find_map(|tt| match tt {
        TokenTree::Group(group) => Some(group.stream().into_iter()),
        _ => None,
    }) else {
        return compile_error("Empty struct is not derivable as Table");
    };
    let mut fields = String::with_capacity(len);
    let mut id: Option<(String, String)> = None;
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Ident(ident) => {
                let field_name = ident.to_string();
                append_col_getter!(field_name, fields);
                goto_next_punct!(',', tokens);
            }
            TokenTree::Punct(punct) if punct.as_char().eq(&'#') => {
                let id_flag_found = tokens
                    .next()
                    .and_then(|tt| {
                        if let TokenTree::Group(g) = tt {
                            Some(g.stream().into_iter())
                        } else {
                            None
                        }
                    })
                    .into_iter()
                    .flatten()
                    .any(|tt| matches!(tt, TokenTree::Ident(ident) if ident.to_string().eq("id")));
                if id.is_some() && id_flag_found {
                    return compile_error("Entity can have only one field flagged as ID");
                }
                let field_name = match tokens.next() {
                    Some(TokenTree::Ident(ident)) => ident.to_string(),
                    // Some(TokenTree::Group(g)) => g.to_string(),
                    _ => unreachable!(),
                };
                goto_next_punct!(':', tokens);
                let field_type = match tokens.next() {
                    Some(TokenTree::Ident(ident)) => ident.to_string(),
                    Some(TokenTree::Group(group)) => group.to_string(),
                    _ => unreachable!(),
                };
                append_col_getter!(field_name, fields);
                id = Some((field_name.to_uppercase(), field_type));
                goto_next_punct!(',', tokens);
            }
            _ => continue,
        }
    }
    let id_dependent_code = match id {
        Some((name, typ)) => format!("
            #[cfg(feature = \"nonblocking\")]
            pub async fn find_by_id(&'a self, id: {typ}) -> ::sibyl::Result<Option<{table_name}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table_name} WHERE {name} = :ID\").await?;
                let rows = stmt.query((\"ID\", id)).await?;
                if let Some(row) = rows.next().await? {{
                    Ok(Some({table_name}  {{ {fields} }}))
                }} else {{
                    Ok(None)
                }}
            }}                    

            #[cfg(feature = \"blocking\")]
            pub fn find_by_id(&'a self, id: {typ}) -> ::sibyl::Result<Option<{table_name}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table_name} WHERE {name} = :ID\")?;
                let rows = stmt.query((\"ID\", id))?;
                if let Some(row) = rows.next()? {{
                    Ok(Some({table_name}  {{ {fields} }}))
                }} else {{
                    Ok(None)
                }}
            }}                    
        "),
        None => "".to_string(),
    };
    let output = format!(
        "       
        pub struct {table_name}Repository<'a> {{
            session: &'a ::sibyl::Session<'a>,
        }}
        
        impl<'a> {table_name}Repository<'a> {{
            pub fn new(session: &'a ::sibyl::Session<'a>) -> Self {{
                Self {{ session }}
            }}
            
            #[cfg(feature = \"nonblocking\")]
            pub async fn find_all(&'a self) -> ::sibyl::Result<Vec<{table_name}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table_name}\").await?;
                let rows = stmt.query(()).await?;
                let mut result = vec![];
                while let Some(row) = rows.next().await? {{
                    result.push(
                        {table_name} {{
                            {fields}  
                        }}
                    );
                }}
                Ok(result)
            }}

            #[cfg(feature = \"blocking\")]
            pub fn find_all(&'a self) -> ::sibyl::Result<Vec<{table_name}>> {{
                let stmt = self.session.prepare(\"SELECT * FROM {table_name}\")?;
                let rows = stmt.query(())?;
                let mut result = vec![];
                while let Some(row) = rows.next()? {{
                    result.push(
                        {table_name} {{
                            {fields}
                        }}
                    );
                }}
            }}

            {id_dependent_code}
        }}"
    );
    output.parse().unwrap()
}
