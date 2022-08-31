use tree_sitter::Parser;

pub fn get_tree(parser_lang: &str, text: &[u8]) -> Option<tree_sitter::Tree> {
    let mut parser = Parser::new();

    match parser_lang {
        "markdown" => {
            parser
                .set_language(tree_sitter_md::language())
                .expect("Could not load markdown grammar");
        }
        "org" => {
            parser
                .set_language(tree_sitter_org::language())
                .expect("Could not load org grammar");
        }
        "restructuredtext" => {
            parser
                .set_language(tree_sitter_rst::language())
                .expect("Could not load restructuredtext grammar");
        }
        _ => {
            return None;
        }
    }

    Some(parser.parse(text, None).expect("Could not parse input"))
}

pub fn get_query(parser_lang: &str) -> Option<tree_sitter::Query> {
    match parser_lang {
        "markdown" => Some(
            tree_sitter::Query::new(
                tree_sitter_md::language(),
                r#"
                    (fenced_code_block
                        (info_string (language) @language)
                        (code_fence_content) @content) @codeblock
                "#,
            )
            .expect("Could not load markdown query"),
        ),
        "org" => Some(
            tree_sitter::Query::new(
                tree_sitter_org::language(),
                r#"
                    (block
                        name: (expr) @_name
                        (#match? @_name "(SRC|src)")
                        parameter: (expr) @language
                        contents: (contents) @content) @codeblock
                "#,
            )
            .expect("Could not load org query"),
        ),
        "restructuredtext" => Some(
            tree_sitter::Query::new(
                tree_sitter_rst::language(),
                r#"
                    (directive
                        name: (type) @_name
                        (#match? @_name "code")
                        body: (body
                            (arguments) @language
                            (content) @content
                            (#offset! @content 0 0 1 0))) @codeblock
                    
                "#,
            )
            .expect("Could not load restructuredtext query"),
        ),
        _ => None,
    }
}

pub fn get_parser_lang_from_filename(filename: &str) -> Option<&str> {
    let filename = filename.to_lowercase();
    if filename.ends_with(".md") {
        return Some("markdown");
    }
    if filename.ends_with(".org") {
        return Some("org");
    }
    if filename.ends_with(".rst") {
        return Some("restructuredtext");
    }
    None
}

pub fn handle_directive(
    directive: &str,
    range: &tree_sitter::Range,
    args: &Vec<tree_sitter::QueryPredicateArg>,
) -> Option<tree_sitter::Range> {
    match directive {
        "offset!" => {
            let start_row_offset = match &args[1] {
                tree_sitter::QueryPredicateArg::String(value) => value.parse::<usize>().unwrap(),
                _ => panic!("Unexpected argument type for offset!"),
            };
            let start_col_offset = match &args[2] {
                tree_sitter::QueryPredicateArg::String(value) => value.parse::<usize>().unwrap(),
                _ => panic!("Unexpected argument type for offset!"),
            };
            let end_row_offset = match &args[3] {
                tree_sitter::QueryPredicateArg::String(value) => value.parse::<usize>().unwrap(),
                _ => panic!("Unexpected argument type for offset!"),
            };
            let end_col_offset = match &args[4] {
                tree_sitter::QueryPredicateArg::String(value) => value.parse::<usize>().unwrap(),
                _ => panic!("Unexpected argument type for offset!"),
            };

            let mut new_range = range.clone();
            new_range.start_point.row = range.start_point.row + start_row_offset;
            new_range.start_point.column = range.start_point.column + start_col_offset;
            new_range.end_point.row = range.end_point.row + end_row_offset;
            new_range.end_point.column = range.end_point.column + end_col_offset;
            return Some(new_range);
        }
        &_ => {}
    }
    None
}
