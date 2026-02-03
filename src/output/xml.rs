use super::{CatOutput, FileOutput, TreeOutput};
use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::error::Error;
use std::io::Cursor;

pub fn output_tree(data: &TreeOutput) -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let codebase = BytesStart::new("codebase");
    writer.write_event(Event::Start(codebase.borrow()))?;

    write_element(&mut writer, "project", &data.project)?;

    let files = BytesStart::new("files");
    writer.write_event(Event::Start(files.borrow()))?;

    for file in &data.files {
        write_file_element(&mut writer, file)?;
    }

    writer.write_event(Event::End(BytesEnd::new("files")))?;
    writer.write_event(Event::End(BytesEnd::new("codebase")))?;

    let result = writer.into_inner().into_inner();
    println!("{}", String::from_utf8(result)?);
    Ok(())
}

pub fn output_cat(data: &CatOutput) -> Result<(), Box<dyn Error>> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let codebase = BytesStart::new("codebase");
    writer.write_event(Event::Start(codebase.borrow()))?;

    write_element(&mut writer, "project", &data.project)?;

    if let Some(ref session) = data.session {
        write_element(&mut writer, "session", session)?;
    }

    write_element(&mut writer, "files_shown", &data.files_shown.to_string())?;
    write_element(
        &mut writer,
        "skipped_binary",
        &data.skipped_binary.to_string(),
    )?;
    write_element(
        &mut writer,
        "skipped_session",
        &data.skipped_session.to_string(),
    )?;
    write_element(&mut writer, "total_lines", &data.total_lines.to_string())?;

    let files = BytesStart::new("files");
    writer.write_event(Event::Start(files.borrow()))?;

    for file in &data.files {
        write_file_element(&mut writer, file)?;
    }

    writer.write_event(Event::End(BytesEnd::new("files")))?;
    writer.write_event(Event::End(BytesEnd::new("codebase")))?;

    let result = writer.into_inner().into_inner();
    println!("{}", String::from_utf8(result)?);
    Ok(())
}

fn write_file_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    file: &FileOutput,
) -> Result<(), Box<dyn Error>> {
    let mut elem = BytesStart::new("file");
    elem.push_attribute(("path", file.path.as_str()));
    elem.push_attribute(("score", file.score.to_string().as_str()));
    elem.push_attribute(("lines", file.lines.to_string().as_str()));

    if let Some(tokens) = file.tokens {
        elem.push_attribute(("tokens", tokens.to_string().as_str()));
    }

    if let Some(ref content) = file.content {
        writer.write_event(Event::Start(elem.borrow()))?;

        let content_elem = BytesStart::new("content");
        writer.write_event(Event::Start(content_elem.borrow()))?;
        writer.write_event(Event::CData(BytesCData::new(content)))?;
        writer.write_event(Event::End(BytesEnd::new("content")))?;

        writer.write_event(Event::End(BytesEnd::new("file")))?;
    } else {
        writer.write_event(Event::Empty(elem))?;
    }

    Ok(())
}

fn write_element<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    let elem = BytesStart::new(name);
    writer.write_event(Event::Start(elem.borrow()))?;
    writer.write_event(Event::Text(BytesText::new(content)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}
