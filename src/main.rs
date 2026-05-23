use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;

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
    kind: String,
    path: String,
    selected: bool,
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
                                                    .find(|entry| entry.selected)
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
        if listing_entry.kind == "directory"
        {
            listing_entry.path.replace("?raw=true", "play.m3u8")
        }
        else
        {
            listing_entry.path.to_string()
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
    let selected_idx = listing_entries.iter().position(|entry| entry.selected).unwrap();
    listing_entries[selected_idx].selected = false;

    let le_len = listing_entries.len();

    listing_entries[
             if next && le_len == selected_idx  + 1 {          0 }
        else if next            { selected_idx  + 1 }
        else if                   selected_idx == 0 { le_len - 1 }
        else                    { selected_idx  - 1 }
    ].selected = true;
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
            if listing_entries[n].selected { SetBackgroundColor(Color::Magenta) }
            else { SetBackgroundColor(Color::Reset) },
        ).unwrap();

        println!("{}{} {}",
            if listing_entries[n].selected { "> " } else { "" },
            match listing_entries[n].kind.as_str() {
                "directory" => "D ",
                "root" => "..",
                "file" => "F ",
                _ => "?"
            },
            // listing_entries[n].path
            stupid_url_fix(&listing_entries[n].path)
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

fn parse_body(body: &str) -> Vec<ListingEntry>
{
    let mut listing_entries = Vec::<ListingEntry>::new();

    let mut buffer  = String::new();
    for character in body.as_bytes().iter()
    {
        match character
        {
            b'>' =>
            {
                if buffer.len() > 2 && &buffer[0..3] == "<a "
                {
                    let class = buffer.split("class=\"").nth(1).unwrap_or("")
                                      .split("\"").nth(0);
                    let href  = buffer.split("href=\"").nth(1).unwrap_or("")
                                      .split("\"").nth(0);

                    if class != None && href != None && class.unwrap() != "root"
                    {
                        listing_entries.push(ListingEntry
                        {
                            kind: class.unwrap().to_string(),
                            path: href.unwrap().to_string(),
                            selected: if listing_entries.len() == 0 { true } else { false },
                        });
                    }
                }
                buffer.clear()
            }
            _ => { buffer.push(*character as char) }
        }
    }

    if listing_entries.len() == 0
    {
        listing_entries.push(ListingEntry
        {
            kind: String::from("error"),
            path: String::from("Nothing here."),
            selected: true,
        });
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
