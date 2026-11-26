mod args;
mod tasks;

use args::Args;

fn main() {
    let args = Args::parse(std::env::args().skip(1));

    // Switch case theo -c trước, các tham số khác xét sau (có thể trùng nhau giữa các task)
    let case = args.get("c").unwrap_or_default();

    let result = match case.as_str() {
        "1" => tasks::crawl_kline::run(&args),
        "" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("[ATOM] Không có task nào ứng với -c {}", other);
            print_help();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("[ATOM] Lỗi: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("ATOM - CLI Tool");
    println!("Cách dùng: cargo run -- -c <case> [tham số của task]");
    println!();
    println!("Danh sách task:");
    println!("  -c 1   Crawl kline Binance (1m) -> parquet (Hive partitioning)");
    println!("         -l  danh sách symbol, vd: BTCUSDT,SUIUSDT,SOLUSDT");
    println!("         -f  từ tháng, format {{year}}{{month}}, vd: 201901");
    println!("         -t  đến tháng, format {{year}}{{month}}, vd: 202610");
    println!("         -d  thư mục lưu parquet, vd: D:\\SpaceA\\store\\crawl");
    println!("         -n  tên file parquet (không cần .parquet), vd: 1");
}
