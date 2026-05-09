use std::io;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crossterm::{
    execute,
    event::{ read, KeyCode },
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
        Clear,
        ClearType,
    },
    cursor::{
        MoveTo,
        Hide,
        Show,
    },
    style::{
        SetBackgroundColor,
        Color,
    }
};

const BASE_URL: &str = "http://192.168.9.1";

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    if let Err(e) = handle_navigation()
    {
        println!("Error: {e:?}\r");
    }
    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;

    Ok(())
}

#[derive(Debug)]
struct ListingEntry
{
    le_type: String,
    le_path: String,
    le_selected: bool,
}

fn handle_navigation() -> Result<(), Box<dyn std::error::Error>>
{
    let body = reqwest::blocking::get(format!("{}/anime/?raw=true", BASE_URL))?.text()?;
    let mut listing_entries = parse_body(body);
    list_entries(&listing_entries);

    while let Ok(event) = read()
    {
        let Some(event) = event.as_key_press_event() else
        {
            continue;
        };

        match event.code
        {
            KeyCode::Esc | KeyCode::Char('q') => { break; }
            KeyCode::Up => { select_entry(false, &mut listing_entries); }
            KeyCode::Down => { select_entry(true, &mut listing_entries); }
            KeyCode::Enter => { launch_or_enter(&mut listing_entries); break; },
            _ => {}
        }

        list_entries(&listing_entries);
    }
    Ok(())
}

fn launch_or_enter(listing_entries: &mut Vec<ListingEntry>)
{
    let selected_entry = listing_entries.iter_mut().find(|entry| entry.le_selected).unwrap();
    let spawn_url = format!(
        "{}{}",
        BASE_URL,
        if selected_entry.le_type == "directory"
        {
            selected_entry.le_path.replace("?raw=true", "play.m3u8")
        }
        else
        {
            selected_entry.le_path.to_string()
        }
    );
    std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "sh" })
                          .arg(if cfg!(target_os = "windows") { "/C"  } else { "-c" })
                          .arg(format!("mpv \"{}\"", spawn_url))
                          .stderr(std::process::Stdio::null())
                          .stdout(std::process::Stdio::null())
                          .stdin(std::process::Stdio::null())
                          .spawn()
                          .expect("mpv spawn failed");
}

fn select_entry(next: bool, listing_entries: &mut Vec<ListingEntry>)
{
    let selected_idx = listing_entries.iter().position(|entry| entry.le_selected).unwrap();
    listing_entries[selected_idx].le_selected = false;

    let le_len = listing_entries.len();

    listing_entries[
             if next && le_len == selected_idx  + 1 {          0 }
        else if next            { selected_idx  + 1 }
        else if                   selected_idx == 0 { le_len - 1 }
        else                    { selected_idx  - 1 }
    ].le_selected = true;
}

fn list_entries(listing_entries: &Vec<ListingEntry>)
{
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        MoveTo(0, 0)
    ).unwrap();

    for n in 0..listing_entries.len()
    {
        execute!(
            io::stdout(),
            MoveTo(0, n as u16),
            Clear(ClearType::CurrentLine),
            if listing_entries[n].le_selected { SetBackgroundColor(Color::Magenta) }
            else { SetBackgroundColor(Color::Reset) },
        ).unwrap();

        println!("{}{} {}",
            if listing_entries[n].le_selected { "> " } else { "" },
            match listing_entries[n].le_type.as_str() {
                "directory" => "D ",
                "root" => "..",
                "file" => "F ",
                _ => "?"
            },
            // listing_entries[n].le_path
            stupid_url_fix(&listing_entries[n].le_path)
        );

        execute!(
            io::stdout(),
            SetBackgroundColor(Color::Reset),
        ).unwrap();
    }
}

fn stupid_url_fix(url: &String) -> String
{
    url.replace("%20", " ")
       .replace("%5B", "[")
       .replace("%5D", "]")
       .replace("%2B", "+")
       .replace("/?raw=true", "")
       .split("/")
       .last()
       .unwrap()
       .to_string()
}

fn parse_body(xml: String) -> Vec<ListingEntry>
{
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut listing_entries = Vec::<ListingEntry>::new();

    loop
    {
        match reader.read_event_into(&mut buf)
        {
            Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),

            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) =>
            {
                match e.name().as_ref()
                {
                    b"a" =>
                    {
                        let kv_vector = e.html_attributes()
                                         .map(|attr|
                                               attr.unwrap()
                                                   .value
                                                   .into_owned()
                                         )
                                         .map(|attr|
                                             String::from_utf8(
                                                 attr
                                             ).unwrap()
                                         )
                                         .collect::<Vec<_>>();

                        if kv_vector[0] == "root" { continue; }

                        listing_entries.push(ListingEntry
                        {
                            le_type: kv_vector[0].clone(),
                            le_path: kv_vector[1].clone(),
                            le_selected: if listing_entries.len() == 0 { true } else { false },
                        });
                    }
                    _ => (),
                }
            }
            _ => (),
        }
        buf.clear();
    }

    listing_entries
}
