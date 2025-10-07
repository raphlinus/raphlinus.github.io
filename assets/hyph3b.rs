// Implementation of Knuth-Liang hyphenation
// Raph Levien (raph@google.com)
// Copyright 2014 Google Inc.

use std::io::{BufferedReader, File};
use std::collections::{HashMap, TreeMap};

struct Trie {
  res: Option<Vec<u8>>,
  succ: TreeMap<char, Box<Trie>>
}

impl Trie {
  fn new() -> Trie {
    Trie { res: None, succ: TreeMap::new()}
  }
  fn insert(&mut self, word: &str, res: Vec<u8>) {
    let leaf = word.chars().fold(self, |node, c| {
      if !node.succ.contains_key(&c) {
        node.succ.insert(c, box Trie::new());
      }
      &mut **node.succ.find_mut(&c).unwrap()
    });
    leaf.res = Some(res)
  }
  fn hyphenate(&self, word: &str) -> Vec<u8> {
    let pw = ".".to_string().append(word).append(".");
    let mut res = Vec::from_elem(pw.len(), 0u8);
    for i in range(0, pw.len()) {
      let mut node = self;
      for j in range(i, pw.len()) {
        let c = pw.as_slice().char_at(j);  // not awesome
        match node.succ.find(&c) {
          None => break,
          Some(next) => node = &**next
        }
        match node.res {
          Some(ref p) => {
            //println!("{} {}", pw.as_slice().slice(i, j + 1), p);
            apply_pattern(&mut res, j + 1, p.as_slice())
          },
          None => ()
        }
      }
    }
    res
  }
  fn add_hyphens(&self, word: &str) -> String {
    let mut ret = String::new();
    let res = self.hyphenate(word);
    for (i, c) in word.chars().enumerate() {
      ret.push_char(c);
      if (res.get(i + 1) & 1) != 0 {
        ret.push_char('-')
      }
    }
    ret
  }
}

struct PatBuilder {
  trie: Trie
}

impl PatBuilder {
  fn new() -> PatBuilder {
    PatBuilder{trie : Trie::new()}
  }
  fn add_pat(&mut self, pat: &str) {
    //let word : String = pat.chars().filter(|c| !c.is_digit_radix(10)).collect();
    let mut res = Vec::new();
    let mut want = false;
    let mut word = String::new();
    for c in pat.chars() {
      match c.to_digit(10) {
        Some(x) => {
          res.push(x as u8);
          want = false;
        },
        None => {
          word.push_char(c);
          if want {
            res.push(0)
          }
          want = true;
        }
      }
    }
    if want { res.push(0) }
    res = res.move_iter().skip_while(|&x| x == 0).collect();;
    //println!("{} {} {}", pat, word, res);
    self.trie.insert(word.as_slice(), res)
  }
}

fn load_pat(filename: &str) -> PatBuilder {
  let mut pb = PatBuilder::new();
  match File::open(&Path::new(filename)) {
    Err(e) => println!("failed to read pattern file, {}", e),
    Ok(v) => for l in BufferedReader::new(v).lines() {
      match l {
        Ok(l) => pb.add_pat(l.as_slice().trim_right_chars('\n')),
        Err(e) => println!("error reading {}: {}", filename, e)
      }
    }
  }
  pb
}

struct MashedEntry {
  res: Vec<u8>,
  res_end: Vec<u8>,
  fail: uint,
  succ: TreeMap<char, uint>
}

struct Mashed {
  arr: Vec<MashedEntry>,
  init: uint,
}

fn apply_pattern(res: &mut Vec<u8>, j: uint, pat: &[u8]) {
  let off = j - pat.len();
  for (k, x) in pat.iter().enumerate() {
    *res.get_mut(off + k) = std::cmp::max(*res.get(off + k), *x);
  }
}

impl Mashed {
  fn hyphenate(&self, word: &str) -> Vec<u8> {
    let mut res = Vec::from_elem(word.len() + 2, 0u8);
    let mut node = self.init;
    for (i, c) in word.chars().enumerate() {
      loop {
        let entry = self.arr.get(node);
        match entry.succ.find(&c) {
          Some(&next) => {
            //println!("{} skip {}", c, entry.res);
            node = next;
            break
          },
          None => {
            //println!("{} @{} {}", c, i, entry.res);
            apply_pattern(&mut res, i + 1, entry.res.as_slice());
            node = entry.fail;
          }
        }
      }
    }
    //println!("final {}", self.arr.get(node).res);
    apply_pattern(&mut res, word.len() + 2, self.arr.get(node).res_end.as_slice());
    res
  }
}

fn fill_rec(m: &mut Mashed, map: &mut HashMap<String, uint>, s: &str, t: &Trie) -> uint {
  let ret = m.arr.len();
  map.insert(s.to_string(), ret);
  m.arr.push(MashedEntry{ res: Vec::new(), res_end: Vec::new(), fail: 0, succ: TreeMap::new() });
  for (&c, &box ref child) in t.succ.iter() {
    let mut child_str = s.to_string();
    child_str.push_char(c);
    let child_ix = fill_rec(m, map, child_str.as_slice(), child);
    m.arr.get_mut(ret).succ.insert(c, child_ix);
  }
  ret
}

fn mash_entry(me: &mut MashedEntry, map: &HashMap<String, uint>, s: &str, root: &Trie) {
  let mut res = Vec::from_elem(s.char_len() + 1, 0 as u8);
  for (i, (i_ix, _c)) in s.char_indices().enumerate() {
    let mut node = root;
    for (j, c) in s.slice_from(i_ix).chars().enumerate() {
      match node.succ.find(&c) {
        None => break,
        Some(next) => node = &**next
      }
      match node.res {
        Some(ref p) => {
          apply_pattern(&mut res, i + j + 2, p.as_slice());
        },
        None => ()
      }
    }
  }
  me.res.extend(res.iter().map(|&x| x).skip_while(|&x| x == 0));
  for (start, _c) in s.char_indices().skip(1) {
    match map.find_equiv(&s.slice_from(start)) {
      Some(&fail) => {
        me.fail = fail;
        break
      },
      None => ()
    }
  }
}

fn prepare_end(m: &mut Mashed, ix: uint) {
  let mut node = ix;
  let mut res = m.arr.get(node).res.clone();
  loop {
    let entry = m.arr.get(node);
    match entry.succ.find(&'.') {
      Some(&next) => {
        node = next;
        break
      },
      None => {
        while res.len() < entry.res.len() {
          res.unshift(0);
        }
        let n = res.len();
        apply_pattern(&mut res, n, entry.res.as_slice());
        node = entry.fail;
      }
    }
  }
  res.push(0);
  {
    let last_res = &m.arr.get(node).res;
    while res.len() < last_res.len() {
      res.unshift(0);
    }
    let n = res.len();
    apply_pattern(&mut res, n, last_res.as_slice());
  }
  m.arr.get_mut(ix).res_end.extend(res.iter().map(|&x| x).skip_while(|&x| x == 0));
}

fn mash(t: &Trie) -> Mashed {
  let mut ret = Mashed { arr: Vec::new(), init: 0 };
  let mut map = HashMap::new();
  fill_rec(&mut ret, &mut map, "", t);
  for (s, &ix) in map.iter() {
    mash_entry(ret.arr.get_mut(ix), &map, s.as_slice(), t);
  }
  for i in range(0, ret.arr.len()) {
    prepare_end(&mut ret, i);
  }
  ret.init = *ret.arr.get(0).succ.find(&'.').unwrap();
  ret
}

fn main() {
  let pb = load_pat("/tmp/hyph-en-us.pat.txt");
  let word = "hyphenation";
  println!("{}", pb.trie.add_hyphens(word));
  println!("{}", pb.trie.hyphenate(word));
  //for _ in range(0u, 1_000_000) {
    //pb.trie.hyphenate(word);
  //}
  let m = mash(&pb.trie);
  println!("{}", m.hyphenate(word));
  for _ in range(0u, 1_000_000) {
    m.hyphenate(word);
  }
}
