//! RDPlatform 入口：加载数据 → 播种 → 启动 HTTP 服务

use rdplatform::api;
use rdplatform::http;
use rdplatform::store::{DEFAULT_DATA_FILE, Store};
use std::cell::RefCell;
use std::net::TcpListener;

fn main() {
    // 加载数据（不存在则首次空库）
    let mut store = match Store::load(DEFAULT_DATA_FILE) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("启动失败: {}", e);
            std::process::exit(1);
        }
    };
    // 首次启动播种演示数据（已有数据不重复播种）
    store.seed();
    if let Err(e) = store.save() {
        eprintln!("保存数据失败: {}", e);
        std::process::exit(1);
    }

    let addr = "127.0.0.1:8080";
    let listener = match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("监听 {} 失败: {}", addr, e);
            std::process::exit(1);
        }
    };

    println!("RDPlatform 研发任务与工时管理平台已启动");
    println!("访问地址: http://{}", addr);
    println!("数据文件: {}", DEFAULT_DATA_FILE);
    println!("演示账号: pm1/pm123（项目经理） dev1/dv123（开发） qa1/qa123（测试）");

    // 单线程顺序处理连接；Store 用 RefCell 提供可变访问
    let store_cell = RefCell::new(store);
    http::serve_loop(listener, move |req| {
        let mut store = store_cell.borrow_mut();
        api::dispatch(req, &mut store)
    });
}
