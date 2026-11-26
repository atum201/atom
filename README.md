# ATOM - CLI Tool

CLI tool viết bằng Rust, thực hiện các task đơn lẻ theo tham số truyền vào.
Cấu trúc switch-case theo `-c`: thêm task mới = thêm module trong `src/tasks/` + thêm 1 case trong `src/main.rs`, không ảnh hưởng code cũ.

## Cách dùng

```
cargo run -- -c <case> [tham số của task]
```

Hỗ trợ cả `-key value` lẫn `--key value`.

## Task 1: Crawl kline Binance (1m)

Crawl nến 1m từ Binance API, lưu parquet theo Hive Partitioning (`year=YYYY/month=MM/`).

| Tham số | Ý nghĩa | Ví dụ |
|---------|---------|-------|
| `-c` | Mã task (1 = crawl kline) | `-c 1` |
| `-l` | Danh sách symbol, phân cách bởi dấu phẩy | `-l BTCUSDT,SUIUSDT,SOLUSDT` |
| `-f` | Từ tháng, format `{year}{month}` | `-f 201901` |
| `-t` | Đến tháng, format `{year}{month}` | `-t 202610` |
| `-d` | Thư mục lưu parquet | `-d D:\SpaceA\store\crawl` |
| `-n` | Tên file parquet (ghi đè nếu đã tồn tại) | `-n 1` |

### Ví dụ lệnh

Crawl 1 symbol trong 1 tháng:

```
cargo run -- -c 1 -l BTCUSDT -f 202610 -t 202610 -d D:\SpaceA\store\crawl -n 1
```

Crawl nhiều symbol trong nhiều tháng (nhóm 1):

```
cargo run -- -c 1 -l BTCUSDT,ETHUSDT,SOLUSDT,SUIUSDT,LINKUSDT,BNBUSDT,XRPUSDT,CRVUSDT,TRXUSDT -f 201901 -t 202610 -d D:\SpaceA\store\crawl -n 1
```

Crawl nhóm coin khác, lưu file riêng cùng partition (nhóm 2 -> `2.parquet`):

```
cargo run -- -c 1 -l SUIUSDT,LINKUSDT,AVAXUSDT -f 202401 -t 202610 -d D:\SpaceA\store\crawl -n 2
```

Crawl lại (ghi đè) dữ liệu 1 tháng của nhóm 1:

```
cargo run -- -c 1 -l BTCUSDT,ETHUSDT,SOLUSDT -f 202609 -t 202609 -d D:\SpaceA\store\crawl -n 1
```

Chạy bản release (nhanh hơn, khuyên dùng khi crawl dài):

```
cargo run --release -- -c 1 -l BTCUSDT,ETHUSDT,SOLUSDT -f 201901 -t 202610 -d D:\SpaceA\store\crawl -n 1
```

Xem help (không truyền `-c`):

```
cargo run
```

Kết quả: mỗi tháng 1 file `{-d}\year=YYYY\month=MM\{-n}.parquet`, dữ liệu đã `ORDER BY symbol, open_time`.

Cột lưu: `symbol, open_time, open, high, low, close, volume, quote_volume, trade, taker_base, taker_quote` (bỏ Close time và Ignore).

## Thêm task mới

1. Tạo `src/tasks/ten_task.rs` với hàm `pub fn run(args: &Args) -> Result<(), Box<dyn Error>>`
2. Khai báo module trong `src/tasks/mod.rs`
3. Thêm case mới vào `match` trong `src/main.rs`
