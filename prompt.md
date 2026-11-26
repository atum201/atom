# ATOM - CLI Tool

- Tạo một CLI Tool viết bằng Rust để thực hiện các task đơn lẻ dựa vào các tham số truyền vào.
- Truyền tham số vào ngày sau: cargo run -- - -list-tham-so
- Có cấu trúc đơn giản và rõ ràng dạng switch case. Sau này nếu thêm task mới chỉ thêm case mới để phân nhánh xử lý mà không ảnh hưởng gì đến code cũ.

# Crawl kline Binance

- link crawl dữ liệu với các tham số
https://api.binance.com/api/v3/klines?symbol=BTCUSDT&interval=1m

- format dữ liệu nhận về
[
  1499040000000,      // Open time -> open_time
  "0.01634790",       // Open -> open
  "0.80000000",       // High -> high
  "0.01575800",       // Low -> low
  "0.01577100",       // Close -> close
  "148976.11427815",  // Volume -> volume
  1499644799999,      // Close time -> dòng này không cần lưu
  "2434.19055334",    // Quote asset volume -> quote_volume
  308,                // Number of trades -> trade
  "1756.87402397",    // Taker buy base asset volume -> taker_base
  "28.46694368",      // Taker buy quote asset volume -> taker_quote
  "17928899.62484339" // Ignore -> bỏ qua không lưu
] 
- dữ liệu nhận về được format như trên, bỏ qua 1 số trường tôi đã đánh dấu: Close time và Ignore. Các trường khác lưu theo tên cột phía sau -> open_time, ... taker_quote. thêm cột symbol để phân biệt từng loại coin

- các tham số truyền vào gồm: ( tôi ko rõ trong rust là truyền -- hay - )
    -c : 1  : case = 1 là mẫ code switch tới task crawl kline binance này. switch case trước khi xét đến các tham số khác, vì có thể trùng nhau
    -l : BTCUSDT,SUIUSDT,LINKUSDT,SOLUSDT .... danh sách các symbol sẽ crawl
    -f -t : from to. tôi sẽ truyền theo format {year}{month} -f 201901 -t 202610 : thời gian crawl từ tháng 1 2019 đến tháng 10 nằm 2026
    -d : thư mục nơi lưu trữ file parquet khi crawl: -d D:\SpaceA\store\crawl
    -n : tên file parquet dữ liệu 1 tháng sẽ lưu : -n 1  : lưu vào file 1.parquet - vì có thể chia thành nhiều nhóm coin crawl khác nhau

- cấu trúc thư mục lưu parquet để hỗ trợ Hive Partitioning. Cấu trúc này sẽ nằm trong thư mục truyền vào qua -d
└── year=2026/
    └── month=01/
        └── 1.parquet
        └── 2.parquet
        
- crawl hết dữ liệu 1m của 1 symbol, rồi crawl lần lượt đến các symbol khác. Sau khi crawl hết thì: ORDER BY symbol, open_time và gộp lại thành 1 file đạt tên theo tham số -n truyền vào. => mỗi 1 tháng và mỗi 1 lần query sẽ sinh ra 1 file có cùng tên qua tham số -n

- lưu ý là mỗi lần call API chỉ trả về 1000 nến, nên cần gộp lại, và tránh thiếu nến. cũng nên wait ngắn để tránh binance giới hạn api khi đang crawl. 
- Sau mỗi lần crawl và ghi file parquet log lên console để quan sát.
- nếu -n là file parquet đã có, nghĩa là tôi muốn ghi đè lại file parquet đó.
- ghi rõ lệnh vào trong README.md

* Cấu trúc cần đơn giản rõ ràng để các loại task khác sau này dễ bô sung vào
