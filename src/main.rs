extern crate hyper;
extern crate regex;
extern crate marksman_escape;
extern crate argparse;

use std::io;
use std::io::Read;
use std::io::Write;
use regex::Regex;
use hyper::Client;
use marksman_escape::Unescape;
use argparse::{ArgumentParser, Store, StoreTrue};

struct SearchResult
{
    hanzi:      String,
    pinyin:     String,
    meaning:    String,
    audio_name: String,
    audio_dir:  String
}

fn main() 
{
    let mut tsv_path = String::new();
    let mut anki_collection_path = String::new();
    let mut query = String::new();
    let mut extended = false;
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut tsv_path).add_argument("tsvPath", Store, "TSV file path").required();
        ap.refer(&mut anki_collection_path).add_argument("ankiCollectionPath", Store, "Anki collection directory").required();
        ap.refer(&mut query).add_argument("pinyin", Store, "Query").required();
        ap.refer(&mut extended).add_option(&["-e", "--extended"], StoreTrue, "Show extended results");
        ap.parse_args_or_exit()
    }
    
    let mut html = get_html(&query);
    if !extended
    {
        html = trim_html(&html).to_string();
    }
    let matches = parse_search_results(&html);
    if matches.len() == 0
    {
        println!("No results found");
        return;
    }

    for (i, m) in matches.iter().take(10).enumerate()
    {
        println!("[{}] {}\t{}\t{}", i, m.hanzi, m.pinyin, m.meaning);
    }

    let number = read_number("Choose: ");
    let item = &matches[number as usize];
    let meaning = read_string("Meaning: ");
    let audio_url = get_download_url(item);
    download_audio(&audio_url, &format!("{}/{}.mp3", anki_collection_path, item.hanzi));
    write_tsv_entry(&item, &tsv_path, &meaning);
    println!("Done!");
}

fn read_string(message: &str) -> String
{
    let stdin = io::stdin();
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut line = String::new();
    stdin.read_line(&mut line).unwrap();
    return line.trim().to_string();
}

fn read_number(message: &str) -> i8
{
    let stdin = io::stdin();

    loop
    {
        print!("{}", message);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        stdin.read_line(&mut line).unwrap();
        match line.trim().parse::<i8>()
        {
            Ok(val) => return val,
            Err(_) => println!("Invalid number"),
        }
    }
}

fn download_audio(url: &str, path: &str)
{
    let client = Client::new();
    let mut res = client.get(url)
        .send()
        .unwrap();

    let mut body = vec!();
    res.read_to_end(&mut body).unwrap();

    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true);

    let file = match options.open(path)
    {
        Ok(val) => val,
        Err(_) => panic!("Unable to open the file \"{}\" for writing", path)
    };
    let mut writer = std::io::BufWriter::new(&file);
    writer.write_all(&body).unwrap();
}

fn write_tsv_entry(item: &SearchResult, path: &str, meaning: &str)
{
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true).append(true);

    let file = match options.open(path)
    {
        Ok(val) => val,
        Err(_) => panic!("Unable to open the file \"{}\" for writing", path)
    };
    let mut writer = std::io::BufWriter::new(&file);
    writer.write(format!("{}\t{} [sound:{}.mp3]\t{}\n", item.hanzi, item.pinyin, item.hanzi, meaning).as_bytes()).unwrap();
}

fn get_download_url(item: &SearchResult) -> String
{
    if item.audio_name.starts_with("word")
    {
        return format!("http://www.trainchinese.com/v1/voicefiles/words_0/{}", item.audio_name);
    }

    let prefix = item.audio_dir
        .parse::<i32>()
        .unwrap() % 1000;

    return format!("http://www.trainchinese.com/v1/word_lists/tc_words/w_dirs/w{}/{}", prefix, item.audio_name);
}

fn parse_search_results(html: &str) -> Vec<SearchResult>
{
    let mut search_results = Vec::new();
    
    let rg = r#"<tr>.+?<div class=['"]leadXXL chinese['"]>(.+?)</div>.+?<span class="pinyin">([^>]+)</span>.+?color:#0066FF['"]> ([^>]+)</span>.+?playAudio\((?:"|&quot;)(.+?)(?:"|&quot;).+?,(\d+)\)"#;
    let re = Regex::new(rg).unwrap();
    let span_re = Regex::new(r#"</?span[^>]*>"#).unwrap();
    for cap in re.captures_iter(html)
    {
        let hanzi = unescape(cap.at(1).unwrap());
        let filtered_hanzi = span_re.replace_all(&hanzi, "").trim().to_string();

        let result = SearchResult {
            hanzi: filtered_hanzi,
            pinyin: unescape(cap.at(2).unwrap()),
            meaning: unescape(cap.at(3).unwrap()),
            audio_name: unescape(cap.at(4).unwrap()),
            audio_dir: unescape(cap.at(5).unwrap())
        };

        search_results.push(result);
    }

    return search_results;
}

fn unescape(string: &str) -> String
{
    return String::from_utf8(Unescape::new(string.bytes()).collect()).unwrap();
}

fn get_html(pinyin: &str) -> String
{
    let url = format!("http://www.trainchinese.com/v2/search.php?searchWord={}&rAp=0&height=0&width=0", pinyin);
    let client = Client::new();
    let mut res = client.get(&url)
        .send()
        .unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    return body;
}

fn trim_html(html: &str) -> &str
{
    let index = match html.find("Showing searches of Pinyin")
        {
            Some(val) => val,
            None => return html
        };

    return &html[index..];
}
