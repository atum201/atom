use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use chrono::{TimeZone, Utc};
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use serde_json::Value;

use crate::args::Args;

const API_URL: &str = "https://api.binance.com/api/v3/klines";
const INTERVAL: &str = "1m";
const LIMIT: u32 = 1000;
const ONE_MINUTE_MS: i64 = 60_000;
/// Nghỉ giữa các lần call API để tránh bị Binance giới hạn
const WAIT_BETWEEN_CALLS_MS: u64 = 300;

/// 1 dòng kline đã parse (bỏ Close time và Ignore)
struct Kline {
    symbol: String,
    open_time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    quote_volume: f64,
    trade: i64,
    taker_base: f64,
    taker_quote: f64,
}

pub fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let symbols: Vec<String> = args
        .require("l")?
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    let from = parse_year_month(&args.require("f")?)?;
    let to = parse_year_month(&args.require("t")?)?;
    let dir = args.require("d")?;
    let name = args.require("n")?;

    if symbols.is_empty() {
        return Err("Danh sách symbol (-l) rỗng".into());
    }
    if from > to {
        return Err("Tham số -f phải <= -t".into());
    }

    println!(
        "[CRAWL] symbols={:?} from={}{:02} to={}{:02} dir={} name={}",
        symbols, from.0, from.1, to.0, to.1, dir, name
    );

    for (year, month) in iter_months(from, to) {
        crawl_month(&symbols, year, month, &dir, &name)?;
    }

    println!("[CRAWL] Hoàn thành toàn bộ.");
    Ok(())
}

/// Crawl toàn bộ symbol trong 1 tháng, sort rồi ghi 1 file parquet
fn crawl_month(
    symbols: &[String],
    year: i32,
    month: u32,
    dir: &str,
    name: &str,
) -> Result<(), Box<dyn Error>> {
    let start_ms = month_start_ms(year, month);
    let end_ms = month_end_ms(year, month);

    println!("[CRAWL] ===== Tháng {}-{:02} =====", year, month);

    let mut rows: Vec<Kline> = Vec::new();
    for symbol in symbols {
        let count = crawl_symbol(symbol, start_ms, end_ms, &mut rows)?;
        println!("[CRAWL] {} {}-{:02}: {} nến", symbol, year, month, count);
    }

    // ORDER BY symbol, open_time
    rows.sort_by(|a, b| {
        a.symbol
            .cmp(&b.symbol)
            .then_with(|| a.open_time.cmp(&b.open_time))
    });

    let path = parquet_path(dir, year, month, name);
    write_parquet(&rows, &path)?;
    println!(
        "[CRAWL] Ghi {} dòng -> {} (ghi đè nếu đã tồn tại)",
        rows.len(),
        path.display()
    );
    Ok(())
}

/// Crawl hết nến 1m của 1 symbol trong khoảng [start_ms, end_ms].
/// Mỗi lần call API tối đa 1000 nến nên phải lặp theo open_time.
fn crawl_symbol(
    symbol: &str,
    start_ms: i64,
    end_ms: i64,
    rows: &mut Vec<Kline>,
) -> Result<usize, Box<dyn Error>> {
    let mut cursor = start_ms;
    let mut count = 0usize;

    while cursor <= end_ms {
        let url = format!(
            "{}?symbol={}&interval={}&startTime={}&endTime={}&limit={}",
            API_URL, symbol, INTERVAL, cursor, end_ms, LIMIT
        );
        let batch = fetch_klines(&url)?;
        if batch.is_empty() {
            break; // hết dữ liệu trong khoảng này
        }

        let mut last_open_time = cursor;
        for item in &batch {
            let k = parse_kline(symbol, item)?;
            last_open_time = k.open_time;
            rows.push(k);
            count += 1;
        }

        // nến cuối cùng chưa chạm cuối tháng -> tiếp tục từ nến kế tiếp
        if last_open_time + ONE_MINUTE_MS <= cursor {
            break; // phòng lặp vô hạn
        }
        cursor = last_open_time + ONE_MINUTE_MS;

        if (batch.len() as u32) < LIMIT {
            break; // API trả ít hơn limit nghĩa là đã hết nến
        }
        thread::sleep(Duration::from_millis(WAIT_BETWEEN_CALLS_MS));
    }

    Ok(count)
}

/// Call API có retry nhẹ khi lỗi mạng / rate limit
fn fetch_klines(url: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut last_err: Option<Box<dyn Error>> = None;
    for attempt in 1..=5 {
        match ureq::get(url).call() {
            Ok(mut resp) => {
                let json: Value = resp.body_mut().read_json()?;
                let arr = json
                    .as_array()
                    .ok_or("Response không phải mảng JSON")?
                    .clone();
                return Ok(arr);
            }
            Err(e) => {
                eprintln!("[CRAWL] Lỗi call API (lần {}): {} -> retry...", attempt, e);
                last_err = Some(Box::new(e));
                thread::sleep(Duration::from_millis(1500 * attempt));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "Call API thất bại".into()))
}

fn parse_kline(symbol: &str, item: &Value) -> Result<Kline, Box<dyn Error>> {
    let arr = item.as_array().ok_or("Kline không phải mảng")?;
    if arr.len() < 11 {
        return Err("Kline thiếu trường".into());
    }
    Ok(Kline {
        symbol: symbol.to_string(),
        open_time: arr[0].as_i64().ok_or("open_time không hợp lệ")?,
        open: parse_f64(&arr[1])?,
        high: parse_f64(&arr[2])?,
        low: parse_f64(&arr[3])?,
        close: parse_f64(&arr[4])?,
        volume: parse_f64(&arr[5])?,
        // arr[6] = Close time -> bỏ qua
        quote_volume: parse_f64(&arr[7])?,
        trade: arr[8].as_i64().ok_or("trade không hợp lệ")?,
        taker_base: parse_f64(&arr[9])?,
        taker_quote: parse_f64(&arr[10])?,
        // arr[11] = Ignore -> bỏ qua
    })
}

fn parse_f64(v: &Value) -> Result<f64, Box<dyn Error>> {
    match v {
        Value::String(s) => Ok(s.parse::<f64>()?),
        Value::Number(n) => n.as_f64().ok_or_else(|| "Số không hợp lệ".into()),
        _ => Err("Giá trị không phải số".into()),
    }
}

/// Đường dẫn Hive Partitioning: {dir}/year=YYYY/month=MM/{name}.parquet
fn parquet_path(dir: &str, year: i32, month: u32, name: &str) -> PathBuf {
    PathBuf::from(dir)
        .join(format!("year={}", year))
        .join(format!("month={:02}", month))
        .join(format!("{}.parquet", name))
}

fn write_parquet(rows: &[Kline], path: &PathBuf) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("symbol", DataType::Utf8, false),
        Field::new("open_time", DataType::Int64, false),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("volume", DataType::Float64, false),
        Field::new("quote_volume", DataType::Float64, false),
        Field::new("trade", DataType::Int64, false),
        Field::new("taker_base", DataType::Float64, false),
        Field::new("taker_quote", DataType::Float64, false),
    ]));

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from_iter_values(
            rows.iter().map(|r| r.symbol.as_str()),
        )),
        Arc::new(Int64Array::from_iter_values(
            rows.iter().map(|r| r.open_time),
        )),
        Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.open))),
        Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.high))),
        Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.low))),
        Arc::new(Float64Array::from_iter_values(rows.iter().map(|r| r.close))),
        Arc::new(Float64Array::from_iter_values(
            rows.iter().map(|r| r.volume),
        )),
        Arc::new(Float64Array::from_iter_values(
            rows.iter().map(|r| r.quote_volume),
        )),
        Arc::new(Int64Array::from_iter_values(rows.iter().map(|r| r.trade))),
        Arc::new(Float64Array::from_iter_values(
            rows.iter().map(|r| r.taker_base),
        )),
        Arc::new(Float64Array::from_iter_values(
            rows.iter().map(|r| r.taker_quote),
        )),
    ];

    let batch = RecordBatch::try_new(schema.clone(), columns)?;

    // File::create ghi đè nếu file đã tồn tại
    let file = fs::File::create(path)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(Default::default()))
        .build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

/// Parse "YYYYMM" -> (year, month)
fn parse_year_month(s: &str) -> Result<(i32, u32), Box<dyn Error>> {
    if s.len() != 6 {
        return Err(format!("Format thời gian phải là YYYYMM, nhận: {}", s).into());
    }
    let year: i32 = s[..4].parse()?;
    let month: u32 = s[4..].parse()?;
    if !(1..=12).contains(&month) {
        return Err(format!("Tháng không hợp lệ: {}", month).into());
    }
    Ok((year, month))
}

/// Danh sách các tháng từ from đến to (bao gồm cả 2 đầu)
fn iter_months(from: (i32, u32), to: (i32, u32)) -> Vec<(i32, u32)> {
    let mut out = Vec::new();
    let (mut y, mut m) = from;
    while (y, m) <= to {
        out.push((y, m));
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    out
}

/// Timestamp ms UTC đầu tháng
fn month_start_ms(year: i32, month: u32) -> i64 {
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis()
}

/// Timestamp ms UTC cuối tháng (ms cuối cùng trước khi sang tháng mới)
fn month_end_ms(year: i32, month: u32) -> i64 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    month_start_ms(ny, nm) - 1
}
