use minify_html::{minify, Cfg};
use std::env;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=templates/");
    println!("cargo:rerun-if-changed=static/");

    if env::var("PROFILE").unwrap() == "release" {
        minify_assets().expect("Failed to minify assets");
    }
}

fn minify_assets() -> Result<(), Box<dyn std::error::Error>> {
    // Create minification config
    let cfg = Cfg {
        do_not_minify_doctype: true,
        ensure_spec_compliant_unquoted_attribute_values: true,
        keep_closing_tags: true,
        keep_html_and_head_opening_tags: true,
        keep_spaces_between_attributes: true,
        minify_css: true,
        minify_js: true,
        remove_bangs: true,
        remove_processing_instructions: true,
        ..Cfg::default()
    };

    // Process HTML templates
    process_templates(&cfg)?;

    // Process static files
    process_static_files()?;

    Ok(())
}

fn process_templates(cfg: &Cfg) -> Result<(), Box<dyn std::error::Error>> {
    let templates_dir = "templates";
    let output_dir = "dist/templates";

    fs::create_dir_all(output_dir)?;

    for entry in fs::read_dir(templates_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("html") {
            let contents = fs::read(&path)?;
            let minified = minify(&contents, cfg);

            let output_path = format!(
                "{}/{}",
                output_dir,
                path.file_name().unwrap().to_str().unwrap()
            );

            fs::write(&output_path, minified)?;
            println!("Minified HTML: {}", output_path);
        }
    }

    Ok(())
}

fn process_static_files() -> Result<(), Box<dyn std::error::Error>> {
    let static_dir = "static";
    let output_dir = "dist/static";

    fs::create_dir_all(format!("{}/css", output_dir))?;
    fs::create_dir_all(format!("{}/js", output_dir))?;
    fs::create_dir_all(format!("{}/js/modules", output_dir))?;

    // Process CSS files
    process_directory(
        &format!("{}/css", static_dir),
        &format!("{}/css", output_dir),
        "css",
    )?;

    // Process JS files
    process_js_files(static_dir, output_dir)?;

    Ok(())
}

fn process_js_files(static_dir: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // First, process module files
    let modules_dir = format!("{}/js/modules", static_dir);
    let modules_output_dir = format!("{}/js/modules", output_dir);

    for entry in fs::read_dir(&modules_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("js") {
            let contents = fs::read_to_string(&path)?;
            let minified = minify_js_module(&contents);

            let output_path = format!(
                "{}/{}",
                modules_output_dir,
                path.file_name().unwrap().to_str().unwrap()
            );

            fs::write(&output_path, minified)?;
            println!("Minified JS Module: {}", output_path);
        }
    }

    // Then process main JS files
    let js_dir = format!("{}/js", static_dir);
    let js_output_dir = format!("{}/js", output_dir);

    for entry in fs::read_dir(&js_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("js") {
            let contents = fs::read_to_string(&path)?;
            let minified = minify_js_module(&contents);

            let output_path = format!(
                "{}/{}",
                js_output_dir,
                path.file_name().unwrap().to_str().unwrap()
            );

            fs::write(&output_path, minified)?;
            println!("Minified JS: {}", output_path);
        }
    }

    Ok(())
}

fn process_directory(
    input_dir: &str,
    output_dir: &str,
    file_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !fs::metadata(input_dir).map(|m| m.is_dir()).unwrap_or(false) {
        return Ok(());
    }

    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let subdir_name = path.file_name().unwrap().to_str().unwrap();
            let new_input_dir = format!("{}/{}", input_dir, subdir_name);
            let new_output_dir = format!("{}/{}", output_dir, subdir_name);
            fs::create_dir_all(&new_output_dir)?;
            process_directory(&new_input_dir, &new_output_dir, file_type)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some(file_type) {
            let contents = fs::read_to_string(&path)?;
            let minified = minify_css(&contents);

            let output_path = format!(
                "{}/{}",
                output_dir,
                path.file_name().unwrap().to_str().unwrap()
            );

            fs::write(&output_path, minified)?;
            println!("Minified {}: {}", file_type.to_uppercase(), output_path);
        }
    }

    Ok(())
}

fn minify_css(content: &str) -> String {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with("/*") && !line.ends_with("*/"))
        .collect::<Vec<&str>>()
        .join("")
        .replace(": ", ":")
        .replace(" {", "{")
        .replace(" }", "}")
        .replace(" ;", ";")
        .replace(", ", ",")
        .replace(" > ", ">")
        .replace(" + ", "+")
        .replace(" ~ ", "~")
}

fn minify_js_module(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut in_comment = false;
    let mut in_regex = false;
    let mut string_char = ' ';
    let mut last_char = ' ';

    while let Some(c) = chars.next() {
        if in_comment {
            if last_char == '*' && c == '/' {
                in_comment = false;
                last_char = ' ';
            } else {
                last_char = c;
            }
            continue;
        }

        if in_string {
            result.push(c);
            if c == string_char && last_char != '\\' {
                in_string = false;
            }
            last_char = c;
            continue;
        }

        if in_regex {
            result.push(c);
            if c == '/' && last_char != '\\' {
                in_regex = false;
            }
            last_char = c;
            continue;
        }

        match c {
            '"' | '\'' | '`' => {
                in_string = true;
                string_char = c;
                result.push(c);
            }
            '/' if last_char == '/' => {
                // Line comment - remove last char and skip until newline
                result.pop();
                while let Some(next) = chars.next() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            '/' if last_char == '*' => {
                // End of block comment
                result.pop();
                in_comment = false;
            }
            '*' if last_char == '/' => {
                // Start of block comment
                result.pop();
                in_comment = true;
            }
            '/' if can_be_regex_start(&result) => {
                in_regex = true;
                result.push(c);
            }
            ' ' | '\t' | '\n' | '\r' => {
                // Only add space if needed
                if !result.ends_with(|c: char| c.is_whitespace())
                    && needs_space(&result, chars.peek().copied())
                {
                    result.push(' ');
                }
            }
            _ => result.push(c),
        }

        last_char = c;
    }

    result.trim().to_string()
}

fn can_be_regex_start(s: &str) -> bool {
    let last_char = s.chars().last();
    match last_char {
        Some(c) => !c.is_alphanumeric() && c != '/' && c != '"' && c != '\'' && c != ')',
        None => true,
    }
}

fn needs_space(before: &str, after: Option<char>) -> bool {
    if before.is_empty() {
        return false;
    }

    let last_char = before.chars().last().unwrap();
    if let Some(next_char) = after {
        // Keep space between words and after keywords
        if last_char.is_alphanumeric() && next_char.is_alphanumeric() {
            return true;
        }

        // Keep space after return
        if before.ends_with("return") && next_char.is_alphanumeric() {
            return true;
        }
    }
    false
}
