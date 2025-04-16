use pyo3::prelude::*;
use pyo3::types::{PyList, PyString};
use scraper::{Html, Selector};
use std::time::Duration;
use futures::future::join_all;
use tokio::time::sleep;

// Pythonに公開する関数
#[pyfunction]
fn get_menu_tables(py: Python) -> PyResult<PyObject> {
    let urls = vec![
        "http://www.gakushoku.com/univ_mn1.php",
        "http://www.gakushoku.com/univ_mn2.php",
    ];

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        get_table(urls).await
    });

    // RustのVec<Vec<Vec<String>>> -> Pythonの list[list[list[str]]]
    let py_tables = PyList::new(py, result.iter().map(|table| {
        PyList::new(py, table.iter().map(|row| {
            PyList::new(py, row.iter().map(|s| PyString::new(py, s)))
        }))
    }));

    Ok(py_tables.into())
}

// HTML取得・パース
async fn get_table(urls: Vec<&str>) -> Vec<Vec<Vec<String>>> {
    let fetches = urls.into_iter().map(|url| {
        async move {
            let mut html = String::new();
            let mut retries = 3;

            while retries > 0 {
                match get_html(&mut html, url).await {
                    Ok(_) => break,
                    Err(_) => {
                        html = String::from("N/A");
                        retries -= 1;
                        sleep(Duration::from_secs(2)).await;
                    }
                }
            }

            html
        }
    });

    let htmls: Vec<String> = join_all(fetches).await;

    htmls
        .into_iter()
        .map(|html| {
            if html != "N/A" {
                parse(html)
            } else {
                vec![vec![String::from("N/A")]]
            }
        })
        .collect()
}

async fn get_html(html: &mut String, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let contents = reqwest::get(url)
        .await?
        .text()
        .await?;
    *html = contents;
    Ok(())
}

fn parse(html: String) -> Vec<Vec<String>> {
    let fragment = Html::parse_fragment(&html);
    let table_selector = Selector::parse("table").unwrap();
    let tr_selector = Selector::parse("tr").unwrap();
    let table_buffer = fragment.select(&table_selector).next();
    if table_buffer.is_none() {
        return vec![vec![String::from("N/A")]];
    }
    let table = table_buffer.unwrap();
    let mut table_row_parsed = Vec::new();

    for tr in table.select(&tr_selector) {
        let mut table_row = tr.text().collect::<Vec<_>>();
        if table_row.len() < 2 { continue; }

        table_row.remove(0);
        table_row.pop();

        for item_idx in 0..table_row.len() {
            if table_row[item_idx].contains("\n") && table_row[item_idx].contains(" ") {
                table_row[item_idx] = "|";
            }
        }

        let mut i = 0;
        while i + 1 < table_row.len() {
            if table_row[i] == "|" && table_row[i + 1] == "|" {
                table_row.insert(i + 1, "None");
                i += 1;
            }
            i += 1;
        }

        table_row_parsed.push(split_and_concat(&table_row, "|"));
    }

    table_row_parsed
}

fn split_and_concat(input: &[&str], separator: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut temp = String::new();

    for item in input.iter() {
        if *item == separator {
            if !temp.is_empty() {
                result.push(temp.clone());
                temp.clear();
            }
        } else {
            temp.push_str(item);
        }
    }

    if !temp.is_empty() {
        result.push(temp);
    }

    result
}

// Python用モジュール定義
#[pymodule]
fn menu_scraper(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_menu_tables, m)?)?;
    Ok(())
}

