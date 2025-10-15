use std::process::Command;
use std::thread;
use std::time::Duration;
use chrono::Local;
use serde::{Deserialize, Serialize};
use tiny_http::{Server, Response, Method};

const HTML_CONTENT: &str = r#"
<!DOCTYPE html> 
<html lang="ko">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>위치 추적</title>
    <style>
        body {
            margin: 0;
            padding: 20px;
            font-family: monospace;
            background: #000;
            color: #0f0;
        }
        #status { font-size: 14px; }
    </style>
</head>
<body>
    <div id="status">위치 추적 중...</div>
    <script>
        window.addEventListener('DOMContentLoaded', () => {
            if (!navigator.geolocation) {
                document.getElementById('status').textContent = '위치 정보 지원 안 됨';
                return;
            }
            navigator.geolocation.getCurrentPosition(
                async (position) => {
                    const data = {
                        latitude: position.coords.latitude,
                        longitude: position.coords.longitude,
                        accuracy: position.coords.accuracy,
                        timestamp: Date.now()
                    };
                    document.getElementById('status').textContent = 
                        `위도: ${data.latitude.toFixed(6)}, 경도: ${data.longitude.toFixed(6)}, 정확도: ${data.accuracy.toFixed(2)}m`;

                    await fetch('/update', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify(data)
                    });

                    // 자동으로 창 닫기
                    setTimeout(() => window.close(), 1500);
                },
                (error) => {
                    document.getElementById('status').textContent = '위치 오류: ' + error.message;
                },
                { enableHighAccuracy: true, timeout: 5000 }
            );
        });
    </script>
</body>
</html>
"#;

#[derive(Debug, Deserialize, Serialize)]
struct LocationData {
    latitude: f64,
    longitude: f64,
    accuracy: f64,
    timestamp: i64,
}

pub fn where_i_am() -> Vec<f64> {
    println!("\n🚀 위치 추적 시스템 시작!");
    println!("🔍 위치 정보를 수집합니다...\n");
    println!("{}", "=".repeat(60));

    // 브라우저 열기
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(500));
        #[cfg(target_os = "windows")]
        Command::new("cmd").args(&["/C", "start", "http://localhost:3030"]).spawn().ok();

        #[cfg(target_os = "macos")]
        Command::new("open").arg("http://localhost:3030").spawn().ok();

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg("http://localhost:3030").spawn().ok();
    });

    let server = Server::http("127.0.0.1:3030").unwrap();
    let mut result: Option<Vec<f64>> = None;

    for mut request in server.incoming_requests() {
        match (request.method(), request.url()) {
            (Method::Get, "/") => {
                let response = Response::from_string(HTML_CONTENT)
                    .with_header(tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap());
                request.respond(response).ok();
            }
            (Method::Post, "/update") => {
                let mut content = String::new();
                request.as_reader().read_to_string(&mut content).ok();

                if let Ok(location) = serde_json::from_str::<LocationData>(&content) {
                    let time = Local::now().format("%Y-%m-%d %H:%M:%S");
                    println!("\n[{}] 📍 새로운 위치 데이터:", time);
                    println!("  위도: {:.8}°", location.latitude);
                    println!("  경도: {:.8}°", location.longitude);
                    println!("  정확도: {:.2}m", location.accuracy);
                    println!(
                        "  Google Maps: https://www.google.com/maps?q={},{}",
                        location.latitude, location.longitude
                    );
                    println!("{}", "=".repeat(60));

                    result = Some(vec![location.latitude, location.longitude, location.accuracy]);

                    let response = Response::from_string(r#"{"status":"ok"}"#)
                        .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap())
                        .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
                    request.respond(response).ok();

                    // 브라우저 닫기 시도
                    #[cfg(target_os = "windows")]
                    Command::new("cmd").args(&["/C", "taskkill /IM chrome.exe /F"]).spawn().ok();

                    #[cfg(target_os = "macos")]
                    Command::new("osascript").args(&["-e", "tell application \"Safari\" to close (every window whose name contains \"위치 추적\")"]).spawn().ok();

                    #[cfg(target_os = "linux")]
                    Command::new("pkill").arg("chrome").spawn().ok();

                    break;
                } else {
                    request.respond(Response::from_string("Invalid JSON").with_status_code(400)).ok();
                }
            }
            (Method::Options, "/update") => {
                let response = Response::empty(200)
                    .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
                    .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Methods", "POST, OPTIONS").unwrap())
                    .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap());
                request.respond(response).ok();
            }
            _ => {
                request.respond(Response::from_string("Not Found").with_status_code(404)).ok();
            }
        }
    }

    // 결과 반환
    result.unwrap_or_else(|| {
        println!("❌ 위치 데이터를 받지 못했습니다.");
        vec![]
    })
}
