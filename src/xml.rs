//! Tokenizador XML mínimo escrito a mano (sin dependencias).
//! Suficiente para document.xml / styles.xml / content.xml. Quita los prefijos de
//! espacio de nombres (w:val -> val) y emite Start+End también para etiquetas vacías.

#[derive(Debug, Clone)]
pub enum Tok {
    Start { name: String, attrs: Vec<(String, String)> },
    End(String),
    Text(String),
}

pub fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn local(name: &str) -> &str {
    match name.rfind(':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

pub fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        match rest.find(';') {
            Some(e) if e <= 10 => {
                let ent = &rest[1..e];
                let rep: Option<char> = match ent {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ if ent.starts_with("#x") || ent.starts_with("#X") => {
                        u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32)
                    }
                    _ if ent.starts_with('#') => ent[1..].parse::<u32>().ok().and_then(char::from_u32),
                    _ => None,
                };
                match rep {
                    Some(c) => {
                        out.push(c);
                        rest = &rest[e + 1..];
                    }
                    None => {
                        out.push('&');
                        rest = &rest[1..];
                    }
                }
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn parse_tag(inner: &str, out: &mut Vec<Tok>) -> String {
    let inner = inner.trim_end();
    let name_end = inner.find(|c: char| c.is_whitespace()).unwrap_or(inner.len());
    let name = local(&inner[..name_end]).to_string();
    let mut attrs: Vec<(String, String)> = Vec::new();
    let rest = &inner[name_end..];
    let rb = rest.as_bytes();
    let mut k = 0;
    while k < rb.len() {
        while k < rb.len() && rb[k].is_ascii_whitespace() {
            k += 1;
        }
        let ks = k;
        while k < rb.len() && rb[k] != b'=' && !rb[k].is_ascii_whitespace() {
            k += 1;
        }
        if ks == k {
            break;
        }
        let key = &rest[ks..k];
        while k < rb.len() && rb[k].is_ascii_whitespace() {
            k += 1;
        }
        if k >= rb.len() || rb[k] != b'=' {
            continue;
        }
        k += 1;
        while k < rb.len() && rb[k].is_ascii_whitespace() {
            k += 1;
        }
        if k >= rb.len() {
            break;
        }
        let q = rb[k];
        if q == b'"' || q == b'\'' {
            k += 1;
            let vs = k;
            while k < rb.len() && rb[k] != q {
                k += 1;
            }
            attrs.push((local(key).to_string(), decode(&rest[vs..k])));
            k += 1;
        } else {
            let vs = k;
            while k < rb.len() && !rb[k].is_ascii_whitespace() {
                k += 1;
            }
            attrs.push((local(key).to_string(), decode(&rest[vs..k])));
        }
    }
    out.push(Tok::Start { name: name.clone(), attrs });
    name
}

pub fn tokenize(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut out: Vec<Tok> = Vec::new();
    while i < b.len() {
        if b[i] == b'<' {
            if src[i..].starts_with("<!--") {
                match src[i + 4..].find("-->") {
                    Some(p) => i += 4 + p + 3,
                    None => break,
                }
                continue;
            }
            if src[i..].starts_with("<![CDATA[") {
                let s = i + 9;
                match src[s..].find("]]>") {
                    Some(p) => {
                        out.push(Tok::Text(src[s..s + p].to_string()));
                        i = s + p + 3;
                    }
                    None => break,
                }
                continue;
            }
            if src[i..].starts_with("<?") {
                match src[i..].find("?>") {
                    Some(p) => i += p + 2,
                    None => break,
                }
                continue;
            }
            if src[i..].starts_with("<!") {
                match src[i..].find('>') {
                    Some(p) => i += p + 1,
                    None => break,
                }
                continue;
            }
            let mut j = i + 1;
            let mut q: u8 = 0;
            while j < b.len() {
                let c = b[j];
                if q != 0 {
                    if c == q {
                        q = 0;
                    }
                } else if c == b'"' || c == b'\'' {
                    q = c;
                } else if c == b'>' {
                    break;
                }
                j += 1;
            }
            if j >= b.len() {
                break;
            }
            let inner = &src[i + 1..j];
            i = j + 1;
            if let Some(rest) = inner.strip_prefix('/') {
                out.push(Tok::End(local(rest.trim()).to_string()));
                continue;
            }
            match inner.strip_suffix('/') {
                Some(x) => {
                    let name = parse_tag(x, &mut out);
                    out.push(Tok::End(name));
                }
                None => {
                    parse_tag(inner, &mut out);
                }
            }
        } else {
            let end = src[i..].find('<').map(|p| i + p).unwrap_or(b.len());
            out.push(Tok::Text(decode(&src[i..end])));
            i = end;
        }
    }
    out
}

/// Cursor consumidor sobre los tokens (cada token se entrega una sola vez).
pub struct Cur {
    t: Vec<Tok>,
    i: usize,
}

impl Cur {
    pub fn new(src: &str) -> Self {
        Cur { t: tokenize(src), i: 0 }
    }

    pub fn next(&mut self) -> Option<Tok> {
        if self.i >= self.t.len() {
            return None;
        }
        let t = std::mem::replace(&mut self.t[self.i], Tok::Text(String::new()));
        self.i += 1;
        Some(t)
    }

    /// Consume hasta cerrar el elemento `name` (se asume que su Start ya fue consumido).
    pub fn skip(&mut self, name: &str) {
        let mut depth = 1;
        while let Some(t) = self.next() {
            match t {
                Tok::Start { name: n, .. } if n == name => depth += 1,
                Tok::End(n) if n == name => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
}
