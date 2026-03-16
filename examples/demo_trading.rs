use std::collections::VecDeque;

use slt::*;

const GREEN: Color = Color::Rgb(0, 192, 135);
const RED: Color = Color::Rgb(246, 70, 93);
const DIM: Color = Color::Indexed(245);
const SURFACE: Color = Color::Indexed(236);

const OB_LEVELS: usize = 12;
const MAX_TRADES: usize = 40;
const MAX_CANDLES: usize = 60;

// ── data types ──────────────────────────────────────────────────

#[derive(Clone)]
struct OB {
    asks: Vec<(f64, f64)>,
    bids: Vec<(f64, f64)>,
}

#[derive(Clone)]
struct Trade {
    time: String,
    price: f64,
    amount: f64,
    is_buy: bool,
}

#[derive(Clone)]
struct Order {
    id: u32,
    side: &'static str,
    otype: &'static str,
    price: f64,
    amount: f64,
    status: &'static str,
}

#[derive(Clone)]
struct Pos {
    symbol: &'static str,
    side: &'static str,
    entry: f64,
    mark: f64,
    size: f64,
    pnl: f64,
}

struct PendingOrder {
    side: &'static str,
    price: f64,
    amount: f64,
    is_limit: bool,
}

struct St {
    pending: Option<PendingOrder>,
    price: f64,
    high24: f64,
    low24: f64,
    vol24: f64,
    candles: Vec<Candle>,
    ob: OB,
    trades: VecDeque<Trade>,
    orders: Vec<Order>,
    positions: Vec<Pos>,
    tab_bottom: TabsState,
    tab_otype: TabsState,
    tab_tf: TabsState,
    inp_price: TextInputState,
    inp_amount: TextInputState,
    tbl_orders: TableState,
    tbl_history: TableState,
    tbl_pos: TableState,
    bal_btc: f64,
    bal_usdt: f64,
    tick: u64,
    candle_interval: u64,
    update_interval: u64,
    frames_acc: u64,
    next_id: u32,
    rng: Rng,
    // per-candle accumulation
    candle_ticks: u64,
    candle_open: f64,
    candle_high: f64,
    candle_low: f64,
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn new(s: u64) -> Self {
        Self(s)
    }
    fn u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn f(&mut self) -> f64 {
        (self.u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f()
    }
    fn coin(&mut self) -> bool {
        (self.u64() & 1) == 0
    }
}

// ── entry ───────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let mut s = St::new();

    slt::run_with(
        RunConfig {
            mouse: true,
            theme: Theme::dark(),
            ..Default::default()
        },
        move |ui: &mut Context| {
            hotkeys(ui, &mut s);
            if let Some(ord) = s.pending.take() {
                submit(&mut s, ord.side, ord.price, ord.amount, ord.is_limit);
            }
            tick(&mut s);

            ui.container().grow(1).gap(0).col(|ui| {
                // header
                header(ui, &s);
                // main 3-col
                ui.container().grow(1).gap(0).row(|ui| {
                    // LEFT: order book
                    ui.bordered(Border::Single)
                        .title("Order Book")
                        .w_pct(25)
                        .col(|ui| {
                            order_book(ui, &s);
                        });
                    // CENTER: chart + trades
                    ui.container().w_pct(50).gap(0).col(|ui| {
                        let old_tf = s.tab_tf.selected;
                        ui.tabs(&mut s.tab_tf);
                        if s.tab_tf.selected != old_tf {
                            s.candle_interval = tf_interval(s.tab_tf.selected);
                            regen_candles(&mut s);
                        }
                        let tf_label =
                            ["1m", "5m", "15m", "1H", "4H", "1D"][s.tab_tf.selected.min(5)];
                        ui.bordered(Border::Single)
                            .title(format!("BTC/USDT {tf_label}"))
                            .grow(2)
                            .col(|ui| {
                                chart(ui, &s);
                            });
                        ui.bordered(Border::Single)
                            .title("Recent Trades")
                            .grow(1)
                            .col(|ui| {
                                trades(ui, &s);
                            });
                    });
                    // RIGHT: order form + balance
                    ui.container().w_pct(25).gap(0).col(|ui| {
                        ui.bordered(Border::Single)
                            .title("Order")
                            .grow(1)
                            .col(|ui| {
                                order_form(ui, &mut s);
                            });
                        ui.bordered(Border::Single).title("Balance").h(6).col(|ui| {
                            balance(ui, &s);
                        });
                    });
                });
                // bottom
                ui.bordered(Border::Single)
                    .title("Orders & Positions")
                    .h(10)
                    .col(|ui| {
                        bottom(ui, &mut s);
                    });
                // status
                status_bar(ui, &s);
            });
        },
    )
}

// ── init ────────────────────────────────────────────────────────

impl St {
    fn new() -> Self {
        let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
        let base = 73_394.0;

        // generate 60 realistic candles with random walk
        let mut candles = Vec::with_capacity(MAX_CANDLES);
        let mut p = base - 800.0; // start ~800 below current
        for _ in 0..MAX_CANDLES {
            let open = p;
            // 4 sub-ticks per candle for realistic OHLC
            let mut hi = open;
            let mut lo = open;
            for _ in 0..4 {
                p *= 1.0 + rng.range(-0.003, 0.0035); // ±0.3% per sub-tick
                hi = hi.max(p);
                lo = lo.min(p);
            }
            candles.push(Candle {
                open,
                high: hi,
                low: lo,
                close: p,
            });
        }
        // snap last candle close to base price
        if let Some(last) = candles.last_mut() {
            last.close = base;
            last.high = last.high.max(base);
        }
        let price = base;

        // pre-generate trades
        let mut trd = VecDeque::with_capacity(30);
        let base_sec: u64 = 12 * 3600 + 40 * 60;
        for i in 0..25 {
            let sec = base_sec.saturating_sub(i * 3);
            let px = price * (1.0 + rng.range(-0.0008, 0.0008));
            trd.push_back(Trade {
                time: fmt_time(sec),
                price: px,
                amount: rng.range(0.002, 0.350),
                is_buy: rng.coin(),
            });
        }

        let orders = vec![
            Order {
                id: 120381,
                side: "BUY",
                otype: "LIMIT",
                price: 72_980.0,
                amount: 0.015,
                status: "NEW",
            },
            Order {
                id: 120382,
                side: "SELL",
                otype: "LIMIT",
                price: 74_220.0,
                amount: 0.010,
                status: "PARTIAL",
            },
            Order {
                id: 120383,
                side: "BUY",
                otype: "LIMIT",
                price: 72_600.0,
                amount: 0.025,
                status: "NEW",
            },
        ];

        let positions = vec![
            Pos {
                symbol: "BTCUSDT",
                side: "LONG",
                entry: 72_140.0,
                mark: price,
                size: 0.042,
                pnl: 0.0,
            },
            Pos {
                symbol: "ETHUSDT",
                side: "SHORT",
                entry: 3_920.0,
                mark: 3_875.0,
                size: 1.2,
                pnl: 54.0,
            },
        ];

        let mut inp_price = TextInputState::with_placeholder("Price");
        inp_price.value = format!("{:.1}", price);
        inp_price.cursor = inp_price.value.chars().count();
        let mut inp_amount = TextInputState::with_placeholder("Amount");
        inp_amount.value = "0.010".into();
        inp_amount.cursor = 5;

        let mut st = Self {
            pending: None,
            price,
            high24: 74_100.0,
            low24: 71_200.0,
            vol24: 28_400_000_000.0,
            candles,
            ob: OB {
                asks: Vec::new(),
                bids: Vec::new(),
            },
            trades: trd,
            orders,
            positions,
            tab_bottom: TabsState::new(vec!["Open Orders", "History", "Positions"]),
            tab_otype: TabsState::new(vec!["Limit", "Market"]),
            tab_tf: TabsState::new(vec!["1m", "5m", "15m", "1H", "4H", "1D"]),
            inp_price,
            inp_amount,
            tbl_orders: TableState::new(
                vec!["ID", "Side", "Type", "Price", "Amount", "Status"],
                Vec::<Vec<String>>::new(),
            ),
            tbl_history: TableState::new(
                vec!["ID", "Side", "Type", "Price", "Amount", "Status"],
                Vec::<Vec<String>>::new(),
            ),
            tbl_pos: TableState::new(
                vec!["Symbol", "Side", "Entry", "Mark", "Size", "PnL"],
                Vec::<Vec<String>>::new(),
            ),
            bal_btc: 0.0234,
            bal_usdt: 1_234.56,
            tick: 0,
            candle_interval: 12,
            update_interval: 30,
            frames_acc: 0,
            next_id: 120400,
            rng,
            candle_ticks: 0,
            candle_open: price,
            candle_high: price,
            candle_low: price,
        };
        regen_ob(&mut st);
        sync_all(&mut st);
        st
    }
}

// ── hotkeys ─────────────────────────────────────────────────────

fn hotkeys(ui: &mut Context, s: &mut St) {
    if ui.key('q') {
        ui.quit();
    }
    if ui.key_code(KeyCode::Esc) {
        ui.quit();
    }
    if ui.key('1') {
        s.tab_bottom.selected = 0;
    }
    if ui.key('2') {
        s.tab_bottom.selected = 1;
    }
    if ui.key('3') {
        s.tab_bottom.selected = 2;
    }
    // speed: [ and ] to avoid conflict with text_input consuming +/-
    if ui.key(']') {
        s.update_interval = s.update_interval.saturating_sub(5).max(5);
    }
    if ui.key('[') {
        s.update_interval = (s.update_interval + 5).min(120);
    }
}

// ── market tick ─────────────────────────────────────────────────

fn tick(s: &mut St) {
    s.frames_acc += 1;
    if s.frames_acc < s.update_interval {
        return;
    }
    s.frames_acc = 0;
    s.tick += 1;

    // price random walk
    let pct = s.rng.range(-0.0012, 0.0012);
    s.price = (s.price * (1.0 + pct)).max(1.0);
    s.high24 = s.high24.max(s.price);
    s.low24 = s.low24.min(s.price);
    s.vol24 += s.rng.range(50_000.0, 250_000.0);

    // candle accumulation
    s.candle_ticks += 1;
    s.candle_high = s.candle_high.max(s.price);
    s.candle_low = s.candle_low.min(s.price);

    if s.candle_ticks >= s.candle_interval {
        // close current candle, push it
        if let Some(last) = s.candles.last_mut() {
            last.close = s.price;
            last.high = s.candle_high;
            last.low = s.candle_low;
        }
        // start new candle
        s.candles.push(Candle {
            open: s.price,
            high: s.price,
            low: s.price,
            close: s.price,
        });
        if s.candles.len() > MAX_CANDLES {
            s.candles.remove(0);
        }
        s.candle_ticks = 0;
        s.candle_open = s.price;
        s.candle_high = s.price;
        s.candle_low = s.price;
    } else if let Some(last) = s.candles.last_mut() {
        // update in-progress candle
        last.close = s.price;
        last.high = s.candle_high;
        last.low = s.candle_low;
    }

    // new trade every other tick
    if s.tick % 2 == 0 {
        let is_buy = s.rng.coin();
        let px = s.price * (1.0 + s.rng.range(-0.0003, 0.0003));
        s.trades.push_front(Trade {
            time: fmt_time(12 * 3600 + 40 * 60 + s.tick * 3),
            price: px,
            amount: s.rng.range(0.001, 0.300),
            is_buy,
        });
        while s.trades.len() > MAX_TRADES {
            s.trades.pop_back();
        }
    }

    // update positions
    for p in &mut s.positions {
        if p.symbol == "BTCUSDT" {
            p.mark = s.price;
            let d = if p.side == "LONG" { 1.0 } else { -1.0 };
            p.pnl = (p.mark - p.entry) * p.size * d;
        }
    }

    regen_ob(s);
    sync_pos(s);
}

fn regen_ob(s: &mut St) {
    s.ob.asks.clear();
    s.ob.bids.clear();
    let half_spread = s.price * 0.00015;
    let step = s.price * 0.00015;
    for i in 0..OB_LEVELS {
        let ap = s.price + half_spread + step * i as f64;
        let bp = s.price - half_spread - step * i as f64;
        s.ob.asks.push((ap, s.rng.range(0.02, 2.0)));
        s.ob.bids.push((bp, s.rng.range(0.02, 2.0)));
    }
}

// ── header ──────────────────────────────────────────────────────

fn header(ui: &mut Context, s: &St) {
    let chg = ((s.price - 73_394.0) / 73_394.0) * 100.0;
    let c = if chg >= 0.0 { GREEN } else { RED };
    let arr = if chg >= 0.0 { "▲" } else { "▼" };
    ui.container().bg(SURFACE).row(|ui| {
        ui.text(" BTC/USDT ").bold();
        ui.text(format!(" ${:.2} ", s.price)).bold().fg(c);
        ui.text(format!("{arr}{:+.2}%", chg)).fg(c);
        ui.spacer();
        ui.text(format!("H:{:.0}", s.high24)).fg(DIM);
        ui.text(format!(" L:{:.0}", s.low24)).fg(DIM);
        ui.text(format!(" Vol:{:.1}B ", s.vol24 / 1e9)).fg(DIM);
    });
}

// ── order book ──────────────────────────────────────────────────

fn order_book(ui: &mut Context, s: &St) {
    // header row
    ui.line(|ui| {
        ui.text("  Price      Qty      Total").fg(DIM);
    });

    let ask_total: f64 = s.ob.asks.iter().map(|a| a.1).sum();
    let bid_total: f64 = s.ob.bids.iter().map(|b| b.1).sum();
    let max_cum = ask_total.max(bid_total).max(0.01);

    // ASKS: accumulate closest→farthest, then display farthest (top) → closest (bottom)
    let mut ask_rows: Vec<(f64, f64, f64)> = Vec::new();
    let mut cum = 0.0;
    for (px, qty) in &s.ob.asks {
        cum += qty;
        ask_rows.push((*px, *qty, cum));
    }
    for (px, qty, total) in ask_rows.iter().rev() {
        let pct = total / max_cum;
        let bar_w = (pct * 12.0).round() as usize;
        let bar: String = " ".repeat(12 - bar_w) + &"█".repeat(bar_w);
        ui.line(|ui| {
            ui.text(format!("{:>10.2}", px)).fg(RED);
            ui.text(format!(" {:>8.4}", qty)).fg(Color::White);
            ui.text(format!(" {:>7.3}", total)).fg(DIM);
            ui.text(bar.clone()).fg(Color::Rgb(60, 20, 25));
        });
    }

    // spread
    let best_a = s.ob.asks.first().map(|a| a.0).unwrap_or(s.price);
    let best_b = s.ob.bids.first().map(|b| b.0).unwrap_or(s.price);
    let sp = (best_a - best_b).max(0.0);
    let sp_pct = sp / s.price * 100.0;
    ui.container().bg(Color::Indexed(234)).row(|ui| {
        ui.text(format!(
            "  ${:.2}  Spread {:.2} ({:.3}%)",
            s.price, sp, sp_pct
        ))
        .bold()
        .fg(Color::Yellow);
    });

    let mut cum = 0.0;
    for (px, qty) in &s.ob.bids {
        cum += qty;
        let pct = cum / max_cum;
        let bar_w = (pct * 12.0).round() as usize;
        let bar: String = "█".repeat(bar_w) + &" ".repeat(12 - bar_w);
        ui.line(|ui| {
            ui.text(format!("{:>10.2}", px)).fg(GREEN);
            ui.text(format!(" {:>8.4}", qty)).fg(Color::White);
            ui.text(format!(" {:>7.3}", cum)).fg(DIM);
            ui.text(bar).fg(Color::Rgb(0, 50, 35));
        });
    }

    // depth ratio bar
    let ratio = bid_total / (ask_total + bid_total).max(0.01);
    ui.line(|ui| {
        ui.text(format!(
            " B:{:.0}% / A:{:.0}%",
            ratio * 100.0,
            (1.0 - ratio) * 100.0
        ))
        .fg(DIM);
    });
}

// ── chart ───────────────────────────────────────────────────────

fn chart(ui: &mut Context, s: &St) {
    if s.candles.is_empty() {
        ui.text("No data").fg(DIM);
        return;
    }
    ui.candlestick(&s.candles, GREEN, RED);
}

// ── trades ──────────────────────────────────────────────────────

fn trades(ui: &mut Context, s: &St) {
    ui.line(|ui| {
        ui.text("  Time       Price       Qty    Side").fg(DIM);
    });
    for t in s.trades.iter().take(10) {
        let c = if t.is_buy { GREEN } else { RED };
        let side = if t.is_buy { "BUY " } else { "SELL" };
        ui.line(|ui| {
            ui.text(format!("  {} ", t.time)).fg(DIM);
            ui.text(format!("{:>10.2} ", t.price)).fg(c);
            ui.text(format!("{:>8.4} ", t.amount)).fg(Color::White);
            ui.text(format!(" {side}")).fg(c).bold();
        });
    }
}

// ── order form ──────────────────────────────────────────────────

fn order_form(ui: &mut Context, s: &mut St) {
    ui.tabs(&mut s.tab_otype);
    let is_limit = s.tab_otype.selected == 0;

    ui.text(" ").fg(DIM); // tiny spacer

    if is_limit {
        ui.text(" Price (USDT)").fg(DIM);
        ui.text_input(&mut s.inp_price);
    } else {
        ui.line(|ui| {
            ui.text(" Price ").fg(DIM);
            ui.text("MARKET").bold().fg(Color::White);
        });
    }

    ui.text(" Amount (BTC)").fg(DIM);
    ui.text_input(&mut s.inp_amount);

    let amount = s.inp_amount.value.trim().parse::<f64>().unwrap_or(0.0);
    let price = if is_limit {
        s.inp_price.value.trim().parse::<f64>().unwrap_or(s.price)
    } else {
        s.price
    };
    let total = amount * price;

    ui.line(|ui| {
        ui.text(" Total: ").fg(DIM);
        ui.text(format!("{total:.2} USDT")).bold();
    });

    ui.text("").fg(DIM);

    let buy_resp = ui.container().bg(GREEN).center().row(|ui| {
        ui.text(" ▲ BUY BTC ").bold().fg(Color::Rgb(0, 0, 0));
    });
    if buy_resp.clicked {
        s.pending = Some(PendingOrder {
            side: "BUY",
            price,
            amount,
            is_limit,
        });
    }

    let sell_resp = ui.container().bg(RED).center().row(|ui| {
        ui.text(" ▼ SELL BTC ").bold().fg(Color::White);
    });
    if sell_resp.clicked {
        s.pending = Some(PendingOrder {
            side: "SELL",
            price,
            amount,
            is_limit,
        });
    }
}

// ── balance ─────────────────────────────────────────────────────

fn balance(ui: &mut Context, s: &St) {
    ui.line(|ui| {
        ui.text(" BTC  ").fg(DIM);
        ui.text(format!("{:.6}", s.bal_btc)).bold();
    });
    ui.line(|ui| {
        ui.text(" USDT ").fg(DIM);
        ui.text(format!("{:.2}", s.bal_usdt)).bold();
    });
    ui.separator();
    let pnl: f64 = s.positions.iter().map(|p| p.pnl).sum();
    let c = if pnl >= 0.0 { GREEN } else { RED };
    ui.line(|ui| {
        ui.text(" PnL ").fg(DIM);
        ui.text(format!("{:+.2}", pnl)).bold().fg(c);
    });
}

// ── bottom panel ────────────────────────────────────────────────

fn bottom(ui: &mut Context, s: &mut St) {
    ui.tabs(&mut s.tab_bottom);
    match s.tab_bottom.selected {
        0 => ui.table(&mut s.tbl_orders),
        1 => ui.table(&mut s.tbl_history),
        _ => ui.table(&mut s.tbl_pos),
    };
}

// ── status bar ──────────────────────────────────────────────────

fn status_bar(ui: &mut Context, s: &St) {
    let ms = 10 + (s.tick % 8);
    let spd = s.update_interval as f64 / 60.0;
    ui.container().bg(SURFACE).row(|ui| {
        ui.text(" ● ").fg(GREEN);
        ui.text("Connected").fg(Color::White);
        ui.text(format!("  {ms}ms")).fg(DIM);
        ui.text(format!("  Speed:{spd:.2}s[ [/] ]")).fg(DIM);
        ui.spacer();
        ui.text(" q:quit  Tab:focus  1/2/3:tabs ").fg(DIM);
    });
}

// ── submit order ────────────────────────────────────────────────

fn submit(s: &mut St, side: &'static str, price: f64, amount: f64, is_limit: bool) {
    if amount <= 0.0 {
        return;
    }
    s.orders.insert(
        0,
        Order {
            id: s.next_id,
            side,
            otype: if is_limit { "LIMIT" } else { "MARKET" },
            price,
            amount,
            status: "NEW",
        },
    );
    s.next_id += 1;
    if s.orders.len() > 20 {
        s.orders.pop();
    }
    sync_orders(s);
}

// ── table sync ──────────────────────────────────────────────────

fn sync_all(s: &mut St) {
    sync_orders(s);
    sync_history(s);
    sync_pos(s);
}

fn sync_orders(s: &mut St) {
    let rows: Vec<Vec<String>> = s
        .orders
        .iter()
        .map(|o| {
            vec![
                o.id.to_string(),
                o.side.into(),
                o.otype.into(),
                format!("{:.1}", o.price),
                format!("{:.4}", o.amount),
                o.status.into(),
            ]
        })
        .collect();
    let mut tbl = TableState::new(
        vec!["ID", "Side", "Type", "Price", "Amount", "Status"],
        rows,
    );
    tbl.page_size = 5;
    s.tbl_orders = tbl;
}

fn sync_history(s: &mut St) {
    let rows: Vec<Vec<String>> = vec![
        vec!["11002", "SELL", "MARKET", "73210.0", "0.0140", "FILLED"],
        vec!["10998", "BUY", "LIMIT", "72880.0", "0.0200", "FILLED"],
        vec!["10974", "SELL", "LIMIT", "74100.0", "0.0100", "CANCELED"],
    ]
    .into_iter()
    .map(|r| r.into_iter().map(String::from).collect())
    .collect();
    let mut tbl = TableState::new(
        vec!["ID", "Side", "Type", "Price", "Amount", "Status"],
        rows,
    );
    tbl.page_size = 5;
    s.tbl_history = tbl;
}

fn sync_pos(s: &mut St) {
    let rows: Vec<Vec<String>> = s
        .positions
        .iter()
        .map(|p| {
            vec![
                p.symbol.into(),
                p.side.into(),
                format!("{:.1}", p.entry),
                format!("{:.1}", p.mark),
                format!("{:.4}", p.size),
                format!("{:+.2}", p.pnl),
            ]
        })
        .collect();
    let mut tbl = TableState::new(vec!["Symbol", "Side", "Entry", "Mark", "Size", "PnL"], rows);
    tbl.page_size = 5;
    s.tbl_pos = tbl;
}

// ── helpers ─────────────────────────────────────────────────────

fn regen_candles(s: &mut St) {
    let volatility = match s.tab_tf.selected {
        0 => 0.002,
        1 => 0.004,
        2 => 0.006,
        3 => 0.010,
        4 => 0.015,
        _ => 0.025,
    };
    let mut candles = Vec::with_capacity(MAX_CANDLES);
    let mut p = s.price - s.price * volatility * 20.0;
    for _ in 0..MAX_CANDLES {
        let open = p;
        let mut hi = open;
        let mut lo = open;
        for _ in 0..6 {
            p *= 1.0 + s.rng.range(-volatility, volatility * 1.1);
            hi = hi.max(p);
            lo = lo.min(p);
        }
        candles.push(Candle {
            open,
            high: hi,
            low: lo,
            close: p,
        });
    }
    if let Some(last) = candles.last_mut() {
        last.close = s.price;
        last.high = last.high.max(s.price);
    }
    s.candles = candles;
    s.candle_ticks = 0;
    s.candle_open = s.price;
    s.candle_high = s.price;
    s.candle_low = s.price;
}

fn tf_interval(selected: usize) -> u64 {
    match selected {
        0 => 12,
        1 => 60,
        2 => 180,
        3 => 720,
        4 => 2880,
        _ => 17280,
    }
}

fn fmt_time(sec: u64) -> String {
    let d = sec % 86_400;
    format!("{:02}:{:02}:{:02}", d / 3600, (d % 3600) / 60, d % 60)
}
