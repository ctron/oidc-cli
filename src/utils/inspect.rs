use biscuit::{Base64Url, CompactPart};
use colored_json::to_colored_json_auto;
use pretty_hex::pretty_hex;
use serde_json::Value;
use std::io::{stdout, Write};

pub fn inspect(token: String) -> anyhow::Result<()> {
    let token = biscuit::Compact::decode(&token);

    for (n, part) in token.parts.into_iter().enumerate() {
        print!(" Part #{n}:");
        if let Err(err) = inspect_part(part) {
            println!("Unable to decode: {err}");
        }
    }

    Ok(())
}

pub fn inspect_part(part: Base64Url) -> anyhow::Result<()> {
    let data = part.to_bytes()?;
    match serde_json::from_slice::<Value>(&data) {
        Err(err) => {
            println!(" Invalid JSON: {err}");
            if is_text(&data) {
                stdout().lock().write_all(&data)?;
            } else {
                let output = pretty_hex(&data);
                stdout().lock().write_all(output.as_bytes())?;
            }
        }
        Ok(value) => {
            println!();
            println!("{}", to_colored_json_auto(&value)?);
        }
    }
    Ok(())
}

fn is_text(data: &[u8]) -> bool {
    String::from_utf8(data.to_vec()).is_ok()
}
