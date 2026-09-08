//! Read-only legacy corpus audit. Requires Node.js for PNG zlib decompression.
//! Run from the workspace root; optional arguments select filenames by prefix.
//! --verify checks legacy fingerprints and wording, reporting title-only English expansions separately.
//! --catalog prints the complete current catalog with refreshed English hashes; never writes files.
//! Prefix filters apply to wording audits, not fingerprint checks; --catalog requires an unfiltered run.
//! The cell layout follows icy_engine's formats/io/icy_draw_v0.rs.
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let status = Command::new("node")
        .args(["-e", SCRIPT, "--"])
        .args(std::env::args().skip(1))
        .status()
        .expect("Node.js is required to decompress the PNG text chunks");
    ExitCode::from(status.code().unwrap_or(1) as u8)
}

const SCRIPT: &str = r###"
const fs = require('fs'), z = require('zlib'), crypto = require('crypto');
const cp = Array.from('ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ');
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const args = process.argv.slice(1);
const verify = args.includes('--verify'), catalog = args.includes('--catalog');
const filters = args.filter(a => !a.startsWith('--'));
for (const arg of args.filter(a => a.startsWith('--'))) {
  if (!['--verify', '--catalog'].includes(arg)) throw Error('Unknown option: ' + arg);
}
if (catalog && (verify || filters.length)) throw Error('--catalog requires an unfiltered run without --verify; it prints all current topics.');
const dataDir = 'crates/icy_board_help/data/';
// Parse only the catalog's single-line JSON-compatible TOML subset; fail closed on new syntax.
function readCatalog() {
  const lines = fs.readFileSync(dataDir + 'catalog.toml', 'utf8').split('\n');
  const topics = new Map(), entries = [];
  let section = {}, topic, version;
  for (const [index, raw] of lines.entries()) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    if (line === '[[topics]]') {
      topic = {fields: {}, translations: {}, hashLine: -1};
      entries.push(topic); section = topic.fields; continue;
    }
    if (line === '[topics.translations.de]') {
      if (!topic || topic.translations.de) throw Error('Invalid or duplicate German metadata table');
      section = topic.translations.de = {}; continue;
    }
    const match = /^([a-z_]+)\s*=\s*(.+)$/.exec(line);
    if (!match) throw Error('Unsupported catalog syntax at line ' + (index + 1));
    const [, key, value] = match;
    const allowed = !topic ? ['version'] : section === topic.fields
      ? ['id', 'locales', 'source_hash', 'legacy_source_hash']
      : ['upstream_hash', 'status', 'legacy_source_hash'];
    if (!allowed.includes(key) || Object.hasOwn(section, key)) throw Error('Unexpected or duplicate catalog key: ' + key);
    try { section[key] = JSON.parse(value); }
    catch { throw Error('Unsupported catalog value at line ' + (index + 1) + '; use a TOML-aware editor.'); }
    if (!topic) version = section[key];
    else if (section === topic.fields && key === 'source_hash') topic.hashLine = index;
  }
  const fingerprint = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
  if (version !== 1 || !entries.length) throw Error('Unsupported or empty catalog');
  for (const entry of entries) {
    const f = entry.fields, de = entry.translations.de;
    if (typeof f.id !== 'string' || !/^hlp[a-z0-9!@]+$/.test(f.id) || topics.has(f.id)) throw Error('Invalid or duplicate topic id');
    if (!Array.isArray(f.locales) || !f.locales.includes('en') || new Set(f.locales).size !== f.locales.length
        || f.locales.some(l => !['en', 'de'].includes(l)) || f.locales.includes('de') !== !!de) throw Error(f.id + ': invalid locales');
    if (entry.hashLine < 0 || !fingerprint(f.source_hash)
        || (Object.hasOwn(f, 'legacy_source_hash') && !fingerprint(f.legacy_source_hash))) throw Error(f.id + ': invalid source fingerprint');
    if (de && (!fingerprint(de.legacy_source_hash) || typeof de.status !== 'string'
        || !(de.upstream_hash === '' || fingerprint(de.upstream_hash)))) throw Error(f.id + ': invalid German metadata');
    topics.set(f.id, entry);
  }
  const sources = fs.readdirSync(dataDir + 'en').filter(name => name.endsWith('.md'));
  if (sources.length !== topics.size || sources.some(name => !topics.has(name.slice(0, -3)))) throw Error('Catalog/English source inventory mismatch');
  return {lines, topics};
}
const current = verify || catalog ? readCatalog() : null;
const records = [];
for (const dir of ['help', 'help_de']) for (const name of fs.readdirSync('crates/icbsetup/data/new_bbs/' + dir).sort()) {
  const bytes = fs.readFileSync('crates/icbsetup/data/new_bbs/' + dir + '/' + name), chunks = [];
  if (bytes.subarray(0,8).toString('hex') !== '89504e470d0a1a0a') throw Error(name + ': not PNG');
  for (let p = 8; p < bytes.length;) {
    const n = bytes.readUInt32BE(p), t = bytes.toString('ascii', p+4, p+8), d = bytes.subarray(p+8,p+8+n);
    if (t === 'zTXt' || t === 'tEXt') {
      const k = d.indexOf(0);
      chunks.push([d.subarray(0,k).toString(), Buffer.from((t === 'zTXt' ? z.inflateSync(d.subarray(k+2)) : d.subarray(k+1)).toString(), 'base64')]);
    }
    p += n+12;
  }
  const header = chunks.find(c => c[0] === 'ICED')[1];
  if (header.length !== 19 || header.readUInt16LE(0) !== 0 || header.readUInt16LE(6) !== 1) throw Error(name + ': unexpected header');
  const layers = [];
  for (const [key,d] of chunks.filter(c => c[0].startsWith('LAYER_'))) {
    if (key.includes('~')) throw Error(name + ': continuation needs review');
    let p = 0;
    const u32 = () => { const v = d.readUInt32LE(p); p += 4; return v; };
    const u16 = () => { const v = d.readUInt16LE(p); p += 2; return v; };
    const len = u32(), title = d.toString('utf8',p,p+len); p += len;
    const role = d[p++]; p += 4;
    const mode = d[p++]; p += 4;
    const flags = u32(); p++;
    const x = u32(), y = u32(), width = u32(), height = u32(); p += 2;
    const length = Number(d.readBigUInt64LE(p)); p += 8;
    if (role || mode || x || y || flags !== 1) throw Error(name + ': unexpected layer properties');
    const lines = [], styles = new Set();
    for (let yy = 0; yy < height && p < d.length; yy++) {
      const cells = [];
      for (let xx = 0; xx < width; xx++) {
        let attr = u16(); if (attr === 0xc000) break;
        const short = !!(attr & 0x4800); attr &= ~0x4800;
        if (attr === 0x8000) { cells.push({ch:' ',fg:7,bg:0}); continue; }
        let ch,fg,bg,font,ext;
        if (short) { ch=d[p++]; fg=d[p++]; bg=d[p++]; font=d[p++]; ext=0; }
        else { ch=u32(); fg=u32(); bg=u32(); font=d[p++]; ext=d[p++]; }
        if (font || ext || attr) throw Error(name + ': unexpected character style');
        styles.add([attr,fg,bg,font,ext].join(':'));
        cells.push({ch:ch >= 128 && ch <= 255 ? cp[ch-128] : String.fromCodePoint(ch),fg,bg});
      }
      lines.push(cells);
    }
    if (p !== d.length) throw Error(name + ': unread layer data');
    layers.push({key,title,role,mode,flags,x,y,width,height,length,styles:[...styles],lines});
  }
  if (layers.length !== 1 || chunks.some(c => !/^(ICED|FONT_0|LAYER_0|END)$/.test(c[0]))) throw Error(name + ': unexpected chunks');
  const record = {name,legacy_sha256:hash(bytes),width:header.readUInt32LE(11),height:header.readUInt32LE(15),chunks:chunks.map(c=>c[0]),layers:layers.map(({lines,...l})=>l)};
  records.push({...record,lines:layers[0].lines.map(line=>line.map(c=>c.ch).join('').trimEnd())});
  if (verify || catalog || (filters.length && !filters.some(a => name.startsWith(a)))) continue;
  console.log(JSON.stringify(record));
  for (const [i,line] of layers[0].lines.entries()) {
    const text = line.map(c=>c.ch).join('').trimEnd();
    console.log(JSON.stringify({line:i+1,text,colors:[...new Set(line.filter(c=>c.ch.trim()).map(c=>c.fg))]}));
  }
}
if (current) {
  const expected = new Map();
  for (const [id, entry] of current.topics) {
    if (entry.fields.legacy_source_hash) expected.set(id + '.icy', entry.fields.legacy_source_hash);
    if (entry.translations.de) expected.set(id + '.de.icy', entry.translations.de.legacy_source_hash);
  }
  for (const r of records) {
    if (expected.get(r.name) !== r.legacy_sha256) throw Error('LEGACY FINGERPRINT MISMATCH ' + r.name + '; review artwork/provenance explicitly.');
    expected.delete(r.name);
  }
  if (expected.size) throw Error('Missing legacy artwork: ' + [...expected.keys()].join(', '));
}
if (verify) {
  let failed = 0, parity = 0;
  const expanded = [];
  const selected = records.filter(r => !filters.length || filters.some(a => r.name.startsWith(a)));
  if (!selected.length) throw Error('No legacy artwork matches the requested prefixes');
  const norm = s => s.replace(/\s+/g,' ').trim();
  for (const r of selected) {
    const locale = r.name.includes('.de.') ? 'de' : 'en', topic = r.name.replace(/(\.de)?\.icy$/,'');
    let source = r.lines.filter((_,i)=>i!==1&&i!==3).map(l=>l.replace(/^│\s*/,'').replace(/\s*│$/,'')).join('\n');
    if (locale === 'de') source = source.replace(/(\p{L})-\n\s*(\p{L})/gu,'$1$2');
    if (topic === 'hlpreg') source = source.replace(/^\s*-+\s*$/mg,'');
    const markdown = fs.readFileSync(dataDir+locale+'/'+topic+'.md','utf8');
    if (locale === 'en' && !r.lines.slice(4).join('').trim()) {
      const lines = markdown.split('\n'), title = /^# (.+)$/.exec(lines[0]);
      const body = lines.slice(1).join('\n').replace(/<!--[\s\S]*?-->/g, '').split('\n')
        .filter(l => !/^\s*#{1,6}(?:\s|$)/.test(l) && !/^\s*```/.test(l)).join('\n');
      if (!title || norm(title[1]) !== norm(source) || !/[\p{L}\p{N}]/u.test(body)) {
        console.error('EXPANSION MISMATCH '+r.name+' (original title and meaningful new body required)'); failed++;
      } else expanded.push(r.name);
      continue;
    }
    let fence = false;
    const plain = markdown.split('\n').map(l=>{
      if (l.startsWith('```')) { fence = !fence; return ''; }
      return fence ? l : l.replace(/^#{1,3} /,'').replace(/^- `([^`]+)` /,'$1 ');
    }).join('\n');
    if (norm(source) !== norm(plain)) { console.error('MISMATCH '+r.name); failed++; }
    else parity++;
  }
  console.log(JSON.stringify({parity,expanded:expanded.length,expanded_files:expanded,failed,total:selected.length,legacy_fingerprints_verified:records.length,english:selected.filter(r=>!r.name.includes('.de.')).length,german:selected.filter(r=>r.name.includes('.de.')).length}));
  console.log(JSON.stringify({dimension_mismatches:records.filter(r=>r.width!==r.layers[0].width||r.height!==r.layers[0].height).map(r=>({file:r.name,canvas:[r.width,r.height],layer:[r.layers[0].width,r.layers[0].height]})),styles:[...new Set(records.flatMap(r=>r.layers[0].styles))],macros:records.flatMap(r=>r.lines.flatMap(l=>l.match(/@[A-Z][^@\n]*@/g)||[])),title_only:records.filter(r=>!r.lines.slice(4).join('').trim()).map(r=>r.name)}));
  if (failed) process.exitCode = 1;
}
if (catalog) {
  for (const [topic, entry] of current.topics) {
    const sourceHash = hash(fs.readFileSync(dataDir+'en/'+topic+'.md'));
    current.lines[entry.hashLine] = current.lines[entry.hashLine].replace(/"[a-f0-9]{64}"/, JSON.stringify(sourceHash));
  }
  process.stdout.write(current.lines.join('\n'));
}
"###;
