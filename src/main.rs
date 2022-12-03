use scraper::{Html, Selector};

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    let body = reqwest::get("https://www.mhlw.go.jp/stf/seisakunitsuite/newpage_00023.html")
        .await?
        .text()
        .await?;
    parse_html(&body);
    Ok(())
}

fn parse_html(text: &str) {
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
    let title: &str = lis
        .next()
        .unwrap()
        .first_child()
        .unwrap()
        .value()
        .as_text()
        .unwrap();
    let link = lis
        .nth(1)
        .unwrap()
        .children()
        .nth(1)
        .unwrap()
        .value()
        .as_element()
        .unwrap()
        .attr("href")
        .unwrap();
    println!("{}", title);
    println!("{}", link);
    // println!(
    //     "{:?}",
    //     link.children()
    //         .take(10)
    //         .filter(|n| n.value().is_element())
    //         .map(|n| n.value())
    // );
    // let title = lis.next().unwrap().value();
}
