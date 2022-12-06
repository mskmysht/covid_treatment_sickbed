use std::{error::Error, ffi::OsStr, fs::File, io::Write, path::Path};

use chrono::{DateTime, TimeZone};
use chrono_tz::{Asia::Tokyo, Tz};
use lazy_regex::regex;
use scraper::{Html, Selector};

#[argopt::cmd]
#[tokio::main]
async fn main(save_to: String, n: Option<usize>) -> Result<(), Box<dyn Error>> {
    let body = reqwest::get("https://www.mhlw.go.jp/stf/seisakunitsuite/newpage_00023.html")
        .await?
        .text()
        .await?;

    for r in parse_html(&body, n) {
        let ext = Path::new(&r.path).extension().unwrap_or(OsStr::new(""));
        let filename = r.timestamp.format("%Y%m%dT%H%M%Z").to_string();
        let data = reqwest::get(format!("https://www.mhlw.go.jp/{}", r.path))
            .await?
            .bytes()
            .await?;
        let path = Path::new(&save_to).join(filename).with_extension(ext);
        if path.exists() {
            println!("[warn] file {} already exists.", path.display());
            continue;
        }
        let mut file = File::create(path)?;
        file.write_all(&data)?;
        println!(
            "[info] report on {} are exported.",
            r.timestamp.format("%Y-%m-%d %H:%M %Z")
        );
    }
    Ok(())
}

struct Report {
    timestamp: DateTime<Tz>,
    path: String,
}

impl Report {
    fn new(timestamp: DateTime<Tz>, path: String) -> Self {
        Self { timestamp, path }
    }
}

const START_DATETIME: &str = "2022-12-23 00:00";

fn parse_html(text: &str, n: Option<usize>) -> Vec<Report> {
    let doc = Html::parse_document(&text);
    let cls = Selector::parse(".m-grid__col1").unwrap();
    let mut lis = doc
        .select(&cls)
        .next()
        .unwrap()
        .first_child()
        .unwrap()
        .children()
        .filter(|n| n.value().is_element());

    let mut dtls = Vec::new();
    let ns: Box<dyn Iterator<Item = usize>> = if let Some(n) = n {
        Box::new(0..n)
    } else {
        Box::new(0..)
    };

    for _ in ns {
        let title = lis
            .next()
            .unwrap()
            .first_child()
            .unwrap()
            .value()
            .as_text()
            .unwrap();
        let dt = extract_datetime(title).unwrap();
        let link = lis
            .nth(1)
            .unwrap()
            .children()
            .filter(|n| n.value().is_element())
            .next()
            .unwrap()
            .value()
            .as_element()
            .unwrap()
            .attr("href")
            .unwrap()
            .to_string();

        dtls.push(Report::new(dt, link.to_string()));
        if dt.format("%Y-%m-%d %H:%M").to_string() == START_DATETIME {
            break;
        }
    }

    dtls
}

fn extract_datetime(title: &str) -> Option<DateTime<Tz>> {
    let re = regex!(
        r"^.*（(?P<year>.+)年(?P<month>.+)月(?P<day>.+)日(?P<hour>.+)時((?P<minute>.+)分|)時点）$"
    );
    let caps = re.captures(&title)?;
    let year = to_half_digits(caps.name("year").unwrap().as_str())?
        .parse()
        .unwrap();
    let month = to_half_digits(caps.name("month").unwrap().as_str())?
        .parse()
        .unwrap();
    let day = to_half_digits(caps.name("day").unwrap().as_str())?
        .parse()
        .unwrap();
    let hour = to_half_digits(caps.name("hour").unwrap().as_str())?
        .parse()
        .unwrap();
    let min = caps
        .name("minute")
        .and_then(|m| to_half_digits(m.as_str()))
        .map(|s| s.parse().unwrap())
        .unwrap_or(0);
    let dt = Tokyo
        .with_ymd_and_hms(year, month, day, hour, min, 0)
        .unwrap();

    Some(dt)
}

fn to_half_digits(fd: &str) -> Option<String> {
    fd.chars()
        .map(|c| {
            match c {
                // convert FULLWIDTH DIGIT ZERO ~ NINE to ascii 0-9
                '\u{FF10}'..='\u{FF19}' => {
                    let k = u32::try_from(c).unwrap() - 0xFF10 + 0x0030;
                    char::from_u32(k)
                }
                '\u{0030}'..='\u{0039}' => Some(c),
                _ => None,
            }
        })
        .collect::<Option<String>>()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono_tz::Asia::Tokyo;

    use crate::extract_datetime;

    const TITLE_A_20221130_0000: &str =
        "入院患者受入病床数等に関する調査結果（2022年11月30日0時時点）";
    const TITLE_A_20221130_0005: &str =
        "入院患者受入病床数等に関する調査結果（2022年11月30日0時5分時点）";
    const TITLE_U_2022_9_5_0000: &str = "（２０２２年９月５日０時時点）";
    const TITLE_U_20220905_0030: &str = "（２０２２年０９月０５日０時３０分時点）";

    #[test]
    fn test_parse_date() {
        let d = extract_datetime(TITLE_A_20221130_0000).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2022-11-30 00:00");

        let d = extract_datetime(TITLE_A_20221130_0005).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2022-11-30 00:05");

        let d = extract_datetime(TITLE_U_2022_9_5_0000).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2022-09-05 00:00");

        let d = extract_datetime(TITLE_U_20220905_0030).unwrap();
        assert_eq!(d.format("%Y-%m-%d %H:%M").to_string(), "2022-09-05 00:30");
    }

    #[test]
    fn test_iso8601() {
        let dt = Tokyo
            .datetime_from_str("20120317T1748JST", "%Y%m%dT%H%M%Z")
            .unwrap();
        println!("{}", dt);
    }
}