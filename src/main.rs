use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crossterm::{
    execute,
    event::{ read, KeyCode },
    terminal::{
        enable_raw_mode, disable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
        Clear, ClearType,
    },
    cursor::{ Show, Hide, MoveTo, },
    style::{ Color, SetBackgroundColor, }
};

const BASE_URL_1: &str = "192.168.1.3";
const BASE_URL_9: &str = "192.168.9.1";

#[derive(Debug)]
struct Router
{
    base: String,
    path: String,
}

impl Router
{
    fn new() -> Router
    {
        Router
        {
            base: String::from(BASE_URL_9),
            path: String::from("anime"),
        }
    }
}

#[derive(Debug)]
struct ListingEntry
{
    le_type: String,
    le_path: String,
    le_selected: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;

    let mut error = None;
    if let Err(e) = handle_navigation()
    {
        error = Some(e);
    }

    execute!(io::stdout(), LeaveAlternateScreen, Show)?;
    disable_raw_mode()?;

    if let Some(e) = error
    {
        println!("Error: {e:?}\r");
    }

    Ok(())
}

fn handle_navigation() -> Result<(), Box<dyn std::error::Error>>
{
    let mut router = Router::new();

    let body = http_get(&router);
    let mut listing_entries = parse_body(&body);

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
            KeyCode::Enter =>
            {
                let selected_entry = listing_entries.iter_mut()
                                                    .find(|entry| entry.le_selected)
                                                    .unwrap();

                launch_or_enter(&selected_entry, &router);
                break;
            },
            KeyCode::Char('s') =>
            {
                router.path = String::from("series");
                let body = http_get(&router);
                listing_entries = parse_body(&body);
            }
            KeyCode::Char('m') =>
            {
                router.path = String::from("movies");
                let body = http_get(&router);
                listing_entries = parse_body(&body);
            }
            KeyCode::Char('a') =>
            {
                router.path = String::from("anime");
                let body = http_get(&router);
                listing_entries = parse_body(&body);
            }
            KeyCode::Char('1') =>
            {
                router.path = String::from("");
                router.base = String::from(BASE_URL_1);
                let body = http_get(&router);
                listing_entries = parse_body(&body);
            }
            KeyCode::Char('9') =>
            {
                router.path = String::from("anime");
                router.base = String::from(BASE_URL_9);
                let body = http_get(&router);
                listing_entries = parse_body(&body);
            }
            _ => {}
        }

        list_entries(&listing_entries);
    }
    Ok(())
}

fn launch_or_enter(listing_entry: &ListingEntry, router: &Router)
{
    let spawn_url = format!(
        "http://{}{}",
        &router.base,
        if listing_entry.le_type == "directory"
        {
            listing_entry.le_path.replace("?raw=true", "play.m3u8")
        }
        else
        {
            listing_entry.le_path.to_string()
        }
    );

    std::process::Command::new("mpv")
                          .arg(spawn_url)
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

fn parse_body(xml: &str) -> Vec<ListingEntry>
{
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut listing_entries = Vec::<ListingEntry>::new();

    loop
    {
        match reader.read_event_into(&mut buf)
        {
            Err(_) => return vec![ListingEntry
            {
                le_type: String::from("error"),
                le_path: String::from("Nothing here."),
                le_selected: true,
            }],
            // Err(e) => return Err(
            //     format!("Error at position {}: {:?}", reader.error_position(), e)
            // ),

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

fn http_get(router: &Router) -> String
{
    let mut stream = match TcpStream::connect(format!("{}:80", &router.base))
    {
        Ok(tcp_stream) => { tcp_stream }
        Err(_) => { return String::from("<whoopsies dayzeyehes<") }
    };

    let request = format!("GET /{}{} ",
                                &router.path,
                                  if &router.path == "" { "" } else { "/?raw=true" }
                         )
                + "HTTP/1.1\r\n"
                + &format!("Host: {}\r\n", &router.base)
                + "Connection: close\r\n"
                + "User-Agent: anipure\r\n\r\n";

    stream.write_all(request.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}
