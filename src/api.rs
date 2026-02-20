use crate::telemetry::TelemetryRow;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

pub fn run_http_server(bind_addr: &str, telemetry: Vec<TelemetryRow>) -> io::Result<()> {
    let listener = TcpListener::bind(bind_addr)?;
    println!("HTTP API listening on http://{bind_addr}");
    serve(listener, telemetry)
}

fn serve(listener: TcpListener, telemetry: Vec<TelemetryRow>) -> io::Result<()> {
    for incoming in listener.incoming() {
        let stream = match incoming {
            Ok(stream) => stream,
            Err(err) => {
                eprintln!("warning: failed to accept connection: {err}");
                continue;
            }
        };

        if let Err(err) = handle_connection(stream, &telemetry) {
            eprintln!("warning: failed to handle request: {err}");
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream, telemetry: &[TelemetryRow]) -> io::Result<()> {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(&mut stream);
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(());
        }

        // Consume and ignore headers.
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            if line == "\r\n" {
                break;
            }
        }
    }

    let request_line = request_line.trim_end_matches(['\r', '\n']);
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    if method != "GET" {
        return write_response(
            &mut stream,
            "405 Method Not Allowed",
            "application/json",
            "{\"error\":\"only GET is supported\"}",
        );
    }

    let (path, query) = split_target(target);
    match path {
        "/state" => {
            if let Some(snapshot) = telemetry.last() {
                let body = serde_json::to_string(snapshot)
                    .map_err(|err| io::Error::other(format!("serialize state: {err}")))?;
                write_response(&mut stream, "200 OK", "application/json", &body)
            } else {
                write_response(
                    &mut stream,
                    "404 Not Found",
                    "application/json",
                    "{\"error\":\"no telemetry available\"}",
                )
            }
        }
        "/telemetry" => {
            let (from, to) = match parse_from_to(query) {
                Ok(range) => range,
                Err(err) => {
                    let body = format!("{{\"error\":\"{err}\"}}");
                    return write_response(
                        &mut stream,
                        "400 Bad Request",
                        "application/json",
                        &body,
                    );
                }
            };
            let rows: Vec<&TelemetryRow> = telemetry
                .iter()
                .filter(|row| {
                    let in_from = from.map(|start| row.timestep >= start).unwrap_or(true);
                    let in_to = to.map(|end| row.timestep <= end).unwrap_or(true);
                    in_from && in_to
                })
                .collect();
            let body = serde_json::to_string(&rows)
                .map_err(|err| io::Error::other(format!("serialize telemetry: {err}")))?;
            write_response(&mut stream, "200 OK", "application/json", &body)
        }
        _ => write_response(
            &mut stream,
            "404 Not Found",
            "application/json",
            "{\"error\":\"not found\"}",
        ),
    }
}

fn split_target(target: &str) -> (&str, &str) {
    if let Some((path, query)) = target.split_once('?') {
        (path, query)
    } else {
        (target, "")
    }
}

fn parse_from_to(query: &str) -> io::Result<(Option<usize>, Option<usize>)> {
    let mut from = None;
    let mut to = None;

    for pair in query.split('&').filter(|entry| !entry.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "from" => {
                from = Some(parse_usize_param("from", value)?);
            }
            "to" => {
                to = Some(parse_usize_param("to", value)?);
            }
            _ => {}
        }
    }

    if let (Some(start), Some(end)) = (from, to) {
        if start > end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "query parameter `from` must be <= `to`",
            ));
        }
    }

    Ok((from, to))
}

fn parse_usize_param(name: &str, value: &str) -> io::Result<usize> {
    value.parse::<usize>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("query parameter `{name}` must be a non-negative integer"),
        )
    })
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

#[cfg(test)]
mod tests {
    use super::parse_from_to;

    #[test]
    fn parses_query_range() {
        assert_eq!(
            parse_from_to("from=1&to=3").expect("query should parse"),
            (Some(1), Some(3))
        );
        assert_eq!(
            parse_from_to("").expect("empty query should parse"),
            (None, None)
        );
    }

    #[test]
    fn rejects_invalid_query_range() {
        assert!(parse_from_to("from=abc").is_err());
        assert!(parse_from_to("from=5&to=1").is_err());
    }
}
