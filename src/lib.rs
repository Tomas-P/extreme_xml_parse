pub mod error;

#[cfg(test)]
mod test;

#[derive(Debug)]
pub enum XmlError {
    /// character not allowed in current parsing context
    BadChar(char),
    /// recursion depth max exceeded
    MaxRecurDepth(u32),
    /// text ends before parsing complete
    TextEnd,
    /// available text does not match any variant of the parsing rule
    NoValidVariant,
    /// illegal substring encountered
    IllegalSubstr,
    /// use of name xml which is reserved
    ReservedNameXml,
    /// mismatch between opening and closing tags
    MismatchedTags(String, String),
    /// did not see opening <![CDATA[ tag while attempting to parse CDSect
    BadCDATAStart,
    /// No available data when trying to parse for character data
    /// Need to make this an error because the rest of the parser doesn't expect
    /// zero-length elements
    NoData,
    /// did not see opening <?xml when attempting to parse XmlDecl
    BadXDeclStart,
    /// did not see a keyword when one was expected
    KeywordMatchFail,
}

trait Ends {
    /// Return the index of the first character that is not part of the node
    /// that occurs after the node
    fn get_endpos(&self) -> usize;
}

impl Ends for Prolog {
    fn get_endpos(&self) -> usize {
        0usize
    }
}

impl Ends for Elem {
    fn get_endpos(&self) -> usize {
        match &self {
            Elem::Empty(empty) => empty.get_endpos(),
            Elem::Full(full) => full.get_endpos(),
        }
    }
}

impl Ends for FullElem {
    fn get_endpos(&self) -> usize {
        self.end.get_endpos()
    }
}

impl Ends for Content {
    fn get_endpos(&self) -> usize {
        match self.items.last() {
            Some(item) => item.get_endpos(),
            None => self.start,
        }
    }
}

impl Ends for Misc {
    fn get_endpos(&self) -> usize {
        match &self {
            &Misc::Ws(ws) => ws.get_endpos(),
            &Misc::Comment(comment) => comment.get_endpos(),
            &Misc::ProcInstr(pi) => pi.get_endpos(),
        }
    }
}

impl Ends for Ws {
    fn get_endpos(&self) -> usize {
        self.start + self.text.len()
    }
}

impl Ends for Comment {
    fn get_endpos(&self) -> usize {
        self.start + self.text.len() + "<!--".len() + "-->".len()
    }
}

impl Ends for ProcInstr {
    fn get_endpos(&self) -> usize {
        let mut endpos = self.start + self.target.name.0.len() + 4;
        match &self.space {
            Some(ws) => {
                endpos += ws.text.len();
            }
            None => (),
        };
        match &self.arg {
            Some(s) => {
                endpos += s.len();
            }
            None => (),
        };
        endpos
    }
}

impl Ends for Attribute {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for AttValue {
    fn get_endpos(&self) -> usize {
        let mut pos = self.start;
        for item in &self.items {
            pos += item.text_len();
        }
        pos + 2 // take qoute chars into account
    }
}

impl Ends for EmptyElem {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for STag {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for ETag {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for ContentItem {
    fn get_endpos(&self) -> usize {
        match &self {
            ContentItem::Elem(elem) => elem.get_endpos(),
            ContentItem::Reference { start, reference } => start + reference.text_len(),
            ContentItem::ProcInstr(pi) => pi.get_endpos(),
            ContentItem::Comment(comment) => comment.get_endpos(),
            ContentItem::CharData(chardata) => chardata.get_endpos(),
            ContentItem::CDSect(cdsect) => cdsect.get_endpos(),
        }
    }
}

impl Ends for CharData {
    fn get_endpos(&self) -> usize {
        self.start + self.text.len()
    }
}

impl Ends for CDSect {
    fn get_endpos(&self) -> usize {
        self.start + "<![CDATA[".len() + self.text.len() + "]]>".len()
    }
}

impl Ends for XmlDecl {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for VersionInfo {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for Encoding {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for SDDecl {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

impl Ends for ExternalID {
    fn get_endpos(&self) -> usize {
        match &self {
            ExternalID::System {
                start,
                end,
                sys_lit,
            } => *end,
            ExternalID::Public {
                start,
                end,
                pub_lit,
                sys_lit,
            } => *end,
        }
    }
}

impl Ends for IntSubsetItem {
    fn get_endpos(&self) -> usize {
        unimplemented!();
    }
}

impl Ends for IntSubset {
    fn get_endpos(&self) -> usize {
        unimplemented!();
    }
}

impl Ends for DoctypeDecl {
    fn get_endpos(&self) -> usize {
        self.end
    }
}

pub struct Doc {
    pub prolog: Prolog,
    pub elem: Elem,
    pub tail: Vec<Misc>,
}

pub fn parse_doc(text: &[char]) -> Result<Doc, XmlError> {
    let prolog = parse_prolog(text, 0)?;
    let p_end = prolog.get_endpos();
    let elem = parse_elem(text, p_end, 0)?;
    let e_end = elem.get_endpos();
    let tail = parse_tail(text, e_end)?;
    let doc = Doc {
        prolog: prolog,
        elem: elem,
        tail: tail,
    };
    Ok(doc)
}

fn parse_prolog(text: &[char], start: usize) -> Result<Prolog, XmlError> {
    let maybe_decl = parse_xmldecl(text, start);
    let (xdecl, pos) = match maybe_decl {
        Ok(xmldecl) => {
            let newpos = xmldecl.get_endpos();
            (Some(xmldecl), newpos)
        }
        Err(_e) => (None, start),
    };
    let mut here = pos;
    let mut miscs = Vec::new();
    while let Ok(misc) = parse_misc(text, here) {
        here = misc.get_endpos();
        miscs.push(misc);
    }
    let maybe_doctypedecl = parse_doctype(text, here);
    let (docdecl, pos1) = match maybe_doctypedecl {
        Ok(doctypedecl) => {
            let newpos = doctypedecl.get_endpos();
            (Some(doctypedecl), newpos)
        }
        Err(_e) => (None, here),
    };
    let mut here2 = pos1;
    while let Ok(misc) = parse_misc(text, here2) {
        here2 = misc.get_endpos();
        miscs.push(misc);
    }
    let prolog = Prolog {
        xml_decl: xdecl,
        doctype_decl: docdecl,
        miscs: miscs,
    };
    Ok(prolog)
}

fn parse_xmldecl(text: &[char], start: usize) -> Result<XmlDecl, XmlError> {
    let subtext = &text[start..];
    let needle: Vec<char> = "<?xml".chars().collect();
    if subtext.starts_with(&needle) {
        let mut here = start + needle.len();
        let version = parse_version(text, here)?;
        here = version.get_endpos();
        let maybe_enc = parse_encoding(text, here);
        let enc = match maybe_enc {
            Ok(encode_decl) => {
                here = encode_decl.get_endpos();
                Some(encode_decl)
            }
            Err(_e) => None,
        };
        let maybe_standalone = parse_standalone(text, here);
        let sddecl = match maybe_standalone {
            Ok(stand) => {
                here = stand.get_endpos();
                Some(stand)
            }
            Err(_e) => None,
        };
        let maybe_space = parse_ws(text, here);
        match maybe_space {
            Ok(ws) => {
                here = ws.get_endpos();
            }
            Err(_e) => (),
        };
        let c_pen = text.get(here).ok_or(XmlError::TextEnd)?;
        if c_pen == &'?' {
            let c_ult = text.get(here + 1).ok_or(XmlError::TextEnd)?;
            if c_ult == &'>' {
                let xmldecl = XmlDecl {
                    start: start,
                    end: here + 2,
                    version: version,
                    encoding: enc,
                    standalone: sddecl,
                };
                Ok(xmldecl)
            } else {
                Err(XmlError::BadChar(*c_ult))
            }
        } else {
            Err(XmlError::BadChar(*c_pen))
        }
    } else {
        Err(XmlError::BadXDeclStart)
    }
}

fn parse_eq(text: &[char], start: usize) -> Result<EqHelper, XmlError> {
    let pos1 = match parse_ws(text, start) {
        Ok(ws) => ws.get_endpos(),
        Err(_e) => start,
    };
    let c1 = text.get(pos1).ok_or(XmlError::TextEnd)?;
    if c1 == &'=' {
        let pos2 = match parse_ws(text, pos1 + 1) {
            Ok(ws) => ws.get_endpos(),
            Err(_e) => pos1 + 1,
        };
        let eq = EqHelper {
            start: start,
            end: pos2,
        };
        Ok(eq)
    } else {
        Err(XmlError::BadChar(*c1))
    }
}

fn parse_standalone(text: &[char], start: usize) -> Result<SDDecl, XmlError> {
    let lead_ws = parse_ws(text, start)?;
    let pos = lead_ws.get_endpos();
    let subtext = &text[pos..];
    let needle: Vec<char> = "standalone".chars().collect();
    if subtext.starts_with(&needle) {
        let pos1 = pos + needle.len();
        let eq = parse_eq(text, pos1)?;
        let pos2 = eq.end;
        let subtext2 = &text[pos2..];
        let needle1: Vec<char> = "\"yes\"".chars().collect();
        let needle2: Vec<char> = "\'yes\'".chars().collect();
        let needle3: Vec<char> = "\"no\"".chars().collect();
        let needle4: Vec<char> = "\'no\'".chars().collect();
        let mut here = pos2;
        let is_standalone = if subtext2.starts_with(&needle1) {
            here += 5;
            true
        } else if subtext2.starts_with(&needle2) {
            here += 5;
            true
        } else if subtext2.starts_with(&needle3) {
            here += 4;
            false
        } else if subtext.starts_with(&needle4) {
            here += 4;
            false
        } else {
            return Err(XmlError::KeywordMatchFail);
        };
        let standalone = SDDecl {
            start: start,
            end: here,
            is_standalone: is_standalone,
        };

        Ok(standalone)
    } else {
        Err(XmlError::KeywordMatchFail)
    }
}

fn parse_encoding(text: &[char], start: usize) -> Result<Encoding, XmlError> {
    let lead_ws = parse_ws(text, start)?;
    let pos = lead_ws.get_endpos();
    let subtext = &text[pos..];
    let needle: Vec<char> = "encoding".chars().collect();
    if subtext.starts_with(&needle) {
        let pos1 = pos + needle.len();
        let eq = parse_eq(text, pos1)?;
        let pos2 = eq.end;
        let c0 = text.get(pos2).ok_or(XmlError::TextEnd)?;
        let single_qoute = c0 == &'\'';
        if c0 == &'"' || single_qoute {
            let mut here = pos2 + 1;
            let mut cur_char = text.get(here).ok_or(XmlError::TextEnd)?;
            let mut arena = String::new();
            let mut first = true;
            while cur_char != c0 {
                if first {
                    match *cur_char {
                        'A'..='Z' | 'a'..='z' => {
                            arena.push(*cur_char);
                        }
                        _ => {
                            return Err(XmlError::BadChar(*cur_char));
                        }
                    };
                } else {
                    match *cur_char {
                        'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' => {
                            arena.push(*cur_char);
                        }
                        _ => {
                            return Err(XmlError::BadChar(*cur_char));
                        }
                    };
                }
                first = false;
                here += 1;
                cur_char = text.get(here).ok_or(XmlError::TextEnd)?;
            }
            let encoding = Encoding {
                start: start,
                end: here + 1,
                enc_name: arena,
            };
            Ok(encoding)
        } else {
            Err(XmlError::BadChar(*c0))
        }
    } else {
        Err(XmlError::KeywordMatchFail)
    }
}

fn parse_version(text: &[char], start: usize) -> Result<VersionInfo, XmlError> {
    let lead_ws = parse_ws(text, start)?;
    let pos = lead_ws.get_endpos();
    let subtext = &text[pos..];
    let needle: Vec<char> = "version".chars().collect();
    if subtext.starts_with(&needle) {
        let pos1 = pos + needle.len();
        let pos2 = match parse_ws(text, pos1) {
            Ok(ws) => ws.get_endpos(),
            Err(_) => pos1,
        };
        let c_eq = text.get(pos2).ok_or(XmlError::TextEnd)?;
        if c_eq == &'=' {
            let pos3 = pos2 + 1;
            let mut here = match parse_ws(text, pos3) {
                Ok(ws) => ws.get_endpos(),
                Err(_) => pos3,
            };
            let c0 = text.get(here).ok_or(XmlError::TextEnd)?;
            let single_qoute = c0 == &'\'';
            if single_qoute || c0 == &'\"' {
                here += 1;
                let mut seen_dot = false;
                let mut arena = String::new();
                let mut cur_char = text.get(here).ok_or(XmlError::TextEnd)?;
                while cur_char != c0 {
                    match *cur_char {
                        '0'..='9' => {
                            arena.push(*cur_char);
                        }
                        '.' => {
                            if !seen_dot {
                                seen_dot = true;
                                arena.push(*cur_char);
                            } else {
                                return Err(XmlError::BadChar(*cur_char));
                            }
                        }
                        _ => {
                            return Err(XmlError::BadChar(*cur_char));
                        }
                    };
                    here += 1;
                    cur_char = text.get(here).ok_or(XmlError::TextEnd)?;
                }
                let maybe_version_num = arena.parse::<f32>();
                let version_num = match maybe_version_num {
                    Ok(num) => num,
                    Err(_e) => {
                        return Err(XmlError::KeywordMatchFail);
                    }
                };
                if version_num >= 2.0 {
                    Err(XmlError::KeywordMatchFail)
                } else {
                    let version_info = VersionInfo {
                        start: start,
                        end: here + 1,
                        ver_num: version_num,
                    };
                    Ok(version_info)
                }
            } else {
                Err(XmlError::BadChar(*c0))
            }
        } else {
            Err(XmlError::BadChar(*c_eq))
        }
    } else {
        Err(XmlError::KeywordMatchFail)
    }
}

fn parse_doctype(text: &[char], start: usize) -> Result<DoctypeDecl, XmlError> {
    let subtext = &text[start..];
    let needle: Vec<char> = "<!DOCTYPE".chars().collect();
    if subtext.starts_with(&needle) {
        let mut here = start + needle.len();
        let spacer1 = parse_ws(text, here)?;
        here = spacer1.get_endpos();
        let name = parse_name(text, here)?;
        here += name.0.len();
        match parse_ws(text, here) {
            Ok(ws) => {here = ws.get_endpos();},
            Err(_e) => (),
        };
        let maybe_extid = parse_externalid(text, here);
        let extid = match maybe_extid {
            Ok(ex_id) => {
                let ending = ex_id.get_endpos();
                here = ending;
                Some(ex_id)
            },
            Err(_e) => None,
        };
        match parse_ws(text, here) {
            Ok(ws) => {here = ws.get_endpos();},
            Err(_e) => (),
        };
        let c0 = *text.get(here).ok_or(XmlError::TextEnd)?;
        if c0 == '[' {
            here += 1;
            let maybe_intsub = parse_intsubset(text, here);
            let intsub = match maybe_intsub {
                Ok(isub) => {
                    here = isub.get_endpos();
                    Some(isub)
                },
                Err(_e) => None,
            };
            let c1 = *text.get(here).ok_or(XmlError::TextEnd)?;
            if c1 == ']' {
                here += 1;
                match parse_ws(text, here) {
                    Ok(ws) => {here = ws.get_endpos();},
                    Err(_e) => (),
                };
                let c2 = *text.get(here).ok_or(XmlError::TextEnd)?;
                if c2 == '>' {
                    let docdecl = DoctypeDecl {
                        start : start,
                        end : here + 1,
                        name : name,
                        ext_id : extid,
                        int_subset : intsub,
                    };
                    Ok(docdecl)
                } else {
                    Err(XmlError::BadChar(c2))
                }
            } else {
                Err(XmlError::BadChar(c1))
            }
        } else if c0 == '>' {
            let docdecl = DoctypeDecl {
                start : start,
                end : here + 1,
                name : name,
                ext_id : extid,
                int_subset : None,
            };
            Ok(docdecl)
        } else {
            Err(XmlError::BadChar(c0))
        }
    } else {
        Err(XmlError::KeywordMatchFail)
    }
}

fn parse_syslit(text: &[char], start: usize) -> Result<String, XmlError> {
    let c0 = text.get(start).ok_or(XmlError::TextEnd)?;
    let single_qoute = c0 == &'\'';
    if c0 == &'\"' || single_qoute {
        let mut here = start + 1;
        let mut arena = String::new();
        while let Some(c) = text.get(here) {
            match *c {
                '\'' => {
                    if single_qoute {
                        return Ok(arena);
                    }
                }
                '\"' => {
                    if !single_qoute {
                        return Ok(arena);
                    }
                }
                _ => (),
            };
            arena.push(*c);
            here += 1;
        }
        Err(XmlError::TextEnd)
    } else {
        Err(XmlError::BadChar(*c0))
    }
}

fn parse_pubidlit(text: &[char], start: usize) -> Result<String, XmlError> {
    let c0 = text.get(start).ok_or(XmlError::TextEnd)?;
    let single_qoute = c0 == &'\'';
    if c0 == &'\"' || single_qoute {
        let mut here = start + 1;
        let mut arena = String::new();
        while let Some(c) = text.get(here) {
            match *c {
                '\'' => {
                    if single_qoute {
                        return Ok(arena);
                    } else {
                        arena.push(*c);
                    }
                }
                '\"' => {
                    if !single_qoute {
                        return Ok(arena);
                    }
                }
                ' '
                | '\r'
                | '\n'
                | 'a'..='z'
                | 'A'..='Z'
                | '0'..='9'
                | '-'
                | '('
                | ')'
                | '+'
                | ','
                | '.'
                | '/'
                | ':'
                | '='
                | '?'
                | ';'
                | '!'
                | '*'
                | '#'
                | '@'
                | '$'
                | '_'
                | '%' => {
                    arena.push(*c);
                }
                _ => {
                    return Err(XmlError::BadChar(*c));
                }
            }
            here += 1;
        }
        Err(XmlError::TextEnd)
    } else {
        Err(XmlError::BadChar(*c0))
    }
}

fn parse_externalid(text: &[char], start: usize) -> Result<ExternalID, XmlError> {
    let subtext = &text[start..];
    let needle1: Vec<char> = "SYSTEM".chars().collect();
    let needle2: Vec<char> = "PUBLIC".chars().collect();
    if subtext.starts_with(&needle1) {
        let pos = start + needle1.len();
        let spacer = parse_ws(text, pos)?;
        let syslit_start = spacer.get_endpos();
        let syslit = parse_syslit(text, syslit_start)?;
        let ext_id = ExternalID::System {
            start: start,
            end: syslit_start + syslit.len() + 2, // account for qoute characters
            sys_lit: syslit,
        };
        Ok(ext_id)
    } else if subtext.starts_with(&needle2) {
        let pos = start + needle2.len();
        let spacer1 = parse_ws(text, pos)?;
        let pubid_start = spacer1.get_endpos();
        let pubid_lit = parse_pubidlit(text, pubid_start)?;
        let pubid_end = pubid_start + pubid_lit.len() + 2; // account for qoute characters
        let spacer2 = parse_ws(text, pubid_end)?;
        let syslit_start = spacer2.get_endpos();
        let syslit = parse_syslit(text, syslit_start)?;
        let ext_id = ExternalID::Public {
            start: start,
            end: syslit_start + syslit.len() + 2, // account for qoute characters
            pub_lit: pubid_lit,
            sys_lit: syslit,
        };
        Ok(ext_id)
    } else {
        Err(XmlError::KeywordMatchFail)
    }
}

fn parse_intsubset(text: &[char], start: usize) -> Result<IntSubset, XmlError> {
    let mut items = Vec::new();
    let mut here = start;
    while let Ok(item) = parse_int_subset_item(text, here) {
        here = item.get_endpos();
        items.push(item);
    }
    if items.len() > 0 {
        let subset = IntSubset {
            items : items,
        };
        Ok(subset)
    } else {
        Err(XmlError::NoData)
    }
}

fn parse_int_subset_item(text :&[char], start :usize) -> Result<IntSubsetItem, XmlError> {
    if let Ok(ws) = parse_ws(text, start) {
        Ok(IntSubsetItem::Blank(ws))
    } else if let Ok(peref) = parse_pereference(text, start) {
        Ok(IntSubsetItem::PEReference(peref))
    } else if let Ok(elemdecl) = parse_elemdecl(text, start) {
        Ok(IntSubsetItem::ElemDecl(elemdecl))
    } else if let Ok(attlist) = parse_attlistdecl(text, start) {
        Ok(IntSubsetItem::AttlistDecl(attlist))
    } else if let Ok(entity) = parse_entitydecl(text, start) {
        Ok(IntSubsetItem::EntityDecl(entity))
    } else if let Ok(notation) = parse_notationdecl(text, start) {
        Ok(IntSubsetItem::NotationDecl(notation))
    } else if let Ok(proc_instr) = parse_pi(text, start) {
        Ok(IntSubsetItem::ProcInstr(proc_instr))
    } else if let Ok(comment) = parse_comment(text, start) {
        Ok(IntSubsetItem::Comment(comment))
    } else {
        Err(XmlError::NoValidVariant)
    }
}

fn parse_notationdecl(text :&[char], start :usize) -> Result<NotationDecl, XmlError> {
    unimplemented!();
}

fn parse_attlistdecl(text :&[char], start :usize) -> Result<AttlistDecl, XmlError> {
    unimplemented!();
}

fn parse_entitydecl(text :&[char], start :usize) -> Result<EntityDecl, XmlError> {
    unimplemented!();
}

fn parse_elemdecl(text :&[char], start :usize) -> Result<ElemDecl, XmlError> {
    unimplemented!();
}

fn parse_pereference(text :&[char], start :usize) -> Result<PEReference, XmlError> {
    let c0 = *text.get(start).ok_or(XmlError::TextEnd)?;
    if c0 == '%' {
        let pos = start + 1;
        let name = parse_name(text, pos)?;
        let pos1 = pos + name.0.len();
        let c1 = *text.get(pos1).ok_or(XmlError::TextEnd)?;
        if c1 == ';' {
            let peref = PEReference(name);
            Ok(peref)
        } else {
            Err(XmlError::BadChar(c1))
        }
    } else {
        Err(XmlError::BadChar(c0))
    }
}

fn parse_tail(text: &[char], start: usize) -> Result<Vec<Misc>, XmlError> {
    let mut buf = Vec::new();
    let mut pos = start;
    let mut maybe_misc = parse_misc(text, pos);
    while let Ok(misc) = maybe_misc {
        pos = misc.get_endpos();
        buf.push(misc);
        maybe_misc = parse_misc(text, pos);
    }
    if let Err(xml_err) = maybe_misc {
        match xml_err {
            XmlError::TextEnd => Ok(buf),
            _ => Err(xml_err),
        }
    } else {
        unreachable!("Should always exhaust XML tail");
    }
}

fn parse_misc(text: &[char], start: usize) -> Result<Misc, XmlError> {
    if let Ok(ws) = parse_ws(text, start) {
        Ok(Misc::Ws(ws))
    } else if let Ok(comment) = parse_comment(text, start) {
        Ok(Misc::Comment(comment))
    } else if let Ok(pi) = parse_pi(text, start) {
        Ok(Misc::ProcInstr(pi))
    } else if text.get(start).is_none() {
        Err(XmlError::TextEnd)
    } else {
        Err(XmlError::NoValidVariant)
    }
}

fn parse_comment(text: &[char], start: usize) -> Result<Comment, XmlError> {
    let char0 = text.get(start).ok_or(XmlError::TextEnd)?;
    if char0 == &'<' {
        let char1 = text.get(start + 1).ok_or(XmlError::TextEnd)?;
        if char1 == &'!' {
            let char2 = text.get(start + 2).ok_or(XmlError::TextEnd)?;
            let char3 = text.get(start + 3).ok_or(XmlError::TextEnd)?;
            if char2 == &'-' && char3 == &'-' {
                let mut buf = String::new();
                let mut count = 0;
                for c in &text[(start + 4)..] {
                    match c {
                        '-' => {
                            count += 1;
                        }
                        '>' => {
                            if count == 2 {
                                match buf.pop() {
                                    Some('-') => (),
                                    Some(c) => {
                                        return Err(XmlError::BadChar(c));
                                    }
                                    None => return Err(XmlError::TextEnd),
                                };
                                match buf.pop() {
                                    Some('-') => (),
                                    Some(c) => {
                                        return Err(XmlError::BadChar(c));
                                    }
                                    None => return Err(XmlError::TextEnd),
                                };
                                let comment = Comment {
                                    start: start,
                                    text: buf,
                                };
                                return Ok(comment);
                            } else if count > 2 {
                                return Err(XmlError::IllegalSubstr);
                            } else {
                                count = 0;
                            }
                        }
                        _ => {
                            if count >= 2 {
                                return Err(XmlError::IllegalSubstr);
                            }
                            count = 0;
                        }
                    };
                    buf.push(*c);
                }
                Err(XmlError::TextEnd)
            } else {
                if char2 == &'-' {
                    Err(XmlError::BadChar(*char3))
                } else {
                    Err(XmlError::BadChar(*char2))
                }
            }
        } else {
            Err(XmlError::BadChar(*char1))
        }
    } else {
        Err(XmlError::BadChar(*char0))
    }
}

fn parse_pi(text: &[char], start: usize) -> Result<ProcInstr, XmlError> {
    let char0 = text.get(start).ok_or(XmlError::TextEnd)?;
    if char0 == &'<' {
        let char1 = text.get(start + 1).ok_or(XmlError::TextEnd)?;
        if char1 == &'?' {
            let target = parse_pitarget(text, start + 2)?;
            let target_end = start + target.name.0.len() + 2;
            let maybe_blank = parse_ws(text, target_end);
            match maybe_blank {
                Ok(ws) => {
                    let blank_end = ws.get_endpos();
                    let mut buf = String::new();
                    let mut seen = false;
                    for c in &text[blank_end..] {
                        match *c {
                            '?' => {
                                seen = true;
                            }
                            '>' => {
                                if seen {
                                    let last = buf.pop().ok_or(XmlError::TextEnd)?;
                                    if last == '?' {
                                        let pi = ProcInstr {
                                            start: start,
                                            target: target,
                                            space: Some(ws),
                                            arg: Some(buf),
                                        };
                                        return Ok(pi);
                                    } else {
                                        return Err(XmlError::BadChar(last));
                                    }
                                }
                            }
                            _ => {
                                seen = false;
                            }
                        };
                        buf.push(*c);
                    }
                    Err(XmlError::TextEnd)
                }
                Err(xml_err) => match xml_err {
                    XmlError::BadChar('?') => {
                        let charlast = text.get(target_end + 1).ok_or(XmlError::TextEnd)?;
                        if charlast == &'>' {
                            let pi = ProcInstr {
                                start: start,
                                target: target,
                                space: None,
                                arg: None,
                            };
                            Ok(pi)
                        } else {
                            Err(XmlError::BadChar(*charlast))
                        }
                    }
                    _ => Err(xml_err),
                },
            }
        } else {
            Err(XmlError::BadChar(*char1))
        }
    } else {
        Err(XmlError::BadChar(*char0))
    }
}

fn parse_pitarget(text: &[char], start: usize) -> Result<PITarget, XmlError> {
    let name = parse_name(text, start)?;
    if name.0.to_lowercase() == "xml" {
        Err(XmlError::ReservedNameXml)
    } else {
        let target = PITarget { name: name };
        Ok(target)
    }
}

fn is_namestart(c: char) -> bool {
    match c {
        ':' | '_' | 'a'..='z' | 'A'..='Z' => true,
        _ => match c as u32 {
            0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x2FF | 0x370..=0x37D => true,
            0x37F..=0x1FFF | 0x200C..=0x200D | 0x2070..=0x218F | 0x2C00..=0x2FEF => true,
            0x3001..=0xD7FF => true,
            0xF900..=0xFDCF | 0xFDF0..=0xFFFD | 0x10000..=0xEFFFF => true,
            _ => false,
        },
    }
}

fn is_namec(c: char) -> bool {
    if is_namestart(c) {
        true
    } else {
        match c {
            '-' | '.' | '0'..='9' => true,
            _ => match c as u32 {
                0xB7 | 0x300..=0x36F | 0x203F..=0x2040 => true,
                _ => false,
            },
        }
    }
}

fn parse_name(text: &[char], start: usize) -> Result<Name, XmlError> {
    let mut buf = String::new();
    let c0 = text.get(start).ok_or(XmlError::TextEnd)?;
    if is_namestart(*c0) {
        buf.push(*c0);
        for c in &text[(start + 1)..] {
            if is_namec(*c) {
                buf.push(*c);
            } else {
                break;
            }
        }
        Ok(Name(buf))
    } else {
        Err(XmlError::BadChar(*c0))
    }
}

fn parse_ws(text: &[char], start: usize) -> Result<Ws, XmlError> {
    let char0 = match text.get(start) {
        Some(c) => c,
        None => {
            return Err(XmlError::TextEnd);
        }
    };
    match char0 {
        ' ' | '\t' | '\n' | '\r' => {
            let mut buf = String::new();
            buf.push(*char0);
            for c in &text[(start + 1)..] {
                match c {
                    ' ' | '\t' | '\n' | '\r' => {
                        buf.push(*c);
                    }
                    _ => break,
                };
            }
            let ws = Ws {
                start: start,
                text: buf,
            };
            Ok(ws)
        }
        _ => {
            return Err(XmlError::BadChar(*char0));
        }
    }
}

fn parse_elem(text: &[char], start: usize, recurdepth: usize) -> Result<Elem, XmlError> {
    let maybe_empty = parse_empty_elem(text, start);
    match maybe_empty {
        Ok(empty) => Ok(Elem::Empty(empty)),
        Err(e) => match e {
            XmlError::TextEnd => Err(e),
            _ => {
                let maybe_full = parse_full_elem(text, start, recurdepth + 1);
                match maybe_full {
                    Ok(full) => Ok(Elem::Full(full)),
                    Err(e) => Err(e),
                }
            }
        },
    }
}

fn parse_empty_elem(text: &[char], start: usize) -> Result<EmptyElem, XmlError> {
    let c0 = text.get(start).ok_or(XmlError::TextEnd)?;
    if c0 == &'<' {
        let name = parse_name(text, start + 1)?;
        let pos = start + 1 + name.0.len();
        let c1 = text.get(pos).ok_or(XmlError::TextEnd)?;
        if c1 == &'/' {
            let c2 = text.get(pos + 1).ok_or(XmlError::TextEnd)?;
            if c2 == &'>' {
                let empty = EmptyElem {
                    start: start,
                    end: pos + 2,
                    name: name,
                    attribs: Vec::new(),
                };
                Ok(empty)
            } else {
                Err(XmlError::BadChar(*c2))
            }
        } else {
            let mut here = pos;
            let mut attribs = Vec::new();
            while text.get(here).ok_or(XmlError::TextEnd)? != &'/' {
                let blank = parse_ws(text, here)?;
                here = blank.get_endpos();
                let maybe_attrib = parse_attribute(text, here);
                match maybe_attrib {
                    Ok(attrib) => {
                        here = attrib.get_endpos();
                        attribs.push(attrib);
                    }
                    Err(e) => match e {
                        XmlError::BadChar('/') => break,
                        _ => return Err(e),
                    },
                };
            }
            let c_here = text.get(here).ok_or(XmlError::TextEnd)?;
            if c_here == &'/' {
                let c_last = text.get(here + 1).ok_or(XmlError::TextEnd)?;
                if c_last == &'>' {
                    let empty = EmptyElem {
                        name: name,
                        start: start,
                        end: here + 2,
                        attribs: attribs,
                    };
                    Ok(empty)
                } else {
                    Err(XmlError::BadChar(*c_last))
                }
            } else {
                Err(XmlError::BadChar(*c_here))
            }
        }
    } else {
        Err(XmlError::BadChar(*c0))
    }
}

fn parse_attribute(text: &[char], start: usize) -> Result<Attribute, XmlError> {
    let name = parse_name(text, start)?;
    let pos = start + name.0.len();
    let maybe_space1 = parse_ws(text, pos);
    let pos1 = match maybe_space1 {
        Ok(ws) => ws.get_endpos(),
        Err(_e) => pos,
    };
    let echar = text.get(pos1).ok_or(XmlError::TextEnd)?;
    if *echar == '=' {
        let maybe_space2 = parse_ws(text, pos1 + 1);
        let pos2 = match maybe_space2 {
            Ok(ws) => ws.get_endpos(),
            Err(_e) => pos1 + 1,
        };
        let value = parse_attvalue(text, pos2)?;
        let attribute = Attribute {
            start: start,
            end: value.get_endpos(),
            name: name,
            value: value,
        };
        Ok(attribute)
    } else {
        Err(XmlError::BadChar(*echar))
    }
}

fn parse_full_elem(text: &[char], start: usize, recurdepth: usize) -> Result<FullElem, XmlError> {
    let start = parse_starttag(text, start)?;
    let pos = start.get_endpos();
    let maybe_content = parse_content(text, pos, recurdepth + 1);
    let mut pos2 = pos;
    let content = match maybe_content {
        Ok(content) => {
            pos2 = content.get_endpos();
            Some(content)
        }
        Err(_e) => None,
    };
    let etag = parse_endtag(text, pos2)?;
    if start.name.0 != etag.name.0 {
        Err(XmlError::MismatchedTags(start.name.0, etag.name.0))
    } else {
        let full = FullElem {
            start: start,
            content: content,
            end: etag,
        };
        Ok(full)
    }
}

fn parse_starttag(text: &[char], start: usize) -> Result<STag, XmlError> {
    let c0 = *text.get(start).ok_or(XmlError::TextEnd)?;
    if c0 == '<' {
        let name = parse_name(text, start + 1)?;
        let pos = start + 1 + name.0.len();
        let c1 = *text.get(pos).ok_or(XmlError::TextEnd)?;
        if c1 == '>' {
            let starttag = STag {
                start: start,
                end: pos + 1,
                name: name,
                attribs: Vec::new(),
            };
            Ok(starttag)
        } else {
            let mut here = pos;
            let mut attribs = Vec::new();
            while text.get(here).ok_or(XmlError::TextEnd)? != &'>' {
                let blank = parse_ws(text, here)?;
                here = blank.get_endpos();
                let maybe_attrib = parse_attribute(text, here);
                match maybe_attrib {
                    Ok(attrib) => {
                        here = attrib.get_endpos();
                        attribs.push(attrib);
                    }
                    Err(e) => match e {
                        XmlError::BadChar('>') => break,
                        _ => {
                            return Err(e);
                        }
                    },
                };
            }
            let c_last = *text.get(here).ok_or(XmlError::TextEnd)?;
            if c_last == '>' {
                let starttag = STag {
                    start: start,
                    end: here + 1,
                    name: name,
                    attribs: attribs,
                };
                Ok(starttag)
            } else {
                Err(XmlError::BadChar(c_last))
            }
        }
    } else {
        Err(XmlError::BadChar(c0))
    }
}

fn parse_content(text: &[char], start: usize, recurdepth: usize) -> Result<Content, XmlError> {
    let mut items = Vec::new();
    let mut position = start;
    while let Ok(item) = parse_content_item(text, position, recurdepth + 1) {
        position = item.get_endpos();
        items.push(item);
    }
    let content = Content {
        start: start,
        items: items,
    };
    Ok(content)
}

fn parse_chardata(text: &[char], start: usize) -> Result<CharData, XmlError> {
    let mut data = String::new();
    let mut count = 0;
    let mut hit_bad_substring = false;
    let mut here = start;

    while !hit_bad_substring {
        let c = text.get(here).ok_or(XmlError::TextEnd)?;
        match *c {
            '<' => {
                if data.len() > 0 {
                    let cdata = CharData {
                        start: start,
                        text: data,
                    };
                    return Ok(cdata);
                } else {
                    return Err(XmlError::NoData);
                }
            }
            '&' => {
                if data.len() > 0 {
                    let cdata = CharData {
                        start: start,
                        text: data,
                    };
                    return Ok(cdata);
                } else {
                    return Err(XmlError::NoData);
                }
            }
            ']' => {
                count += 1;
                data.push(*c);
            }
            '>' => {
                if count >= 2 {
                    hit_bad_substring = true;
                    here += 1;
                    continue;
                } else {
                    count = 0;
                    data.push(*c);
                }
            }
            _ => {
                count = 0;
                data.push(*c);
            }
        };
        here += 1;
    }
    Err(XmlError::IllegalSubstr)
}

fn parse_cdsect(text: &[char], start: usize) -> Result<CDSect, XmlError> {
    let subtext = &text[start..];
    let start_needle: Vec<char> = "<![CDATA[".chars().collect();
    if subtext.starts_with(&start_needle) {
        let pos = start + start_needle.len();
        let mut count = 0;
        let mut data = String::new();
        for c in &text[pos..] {
            match *c {
                ']' => {
                    count += 1;
                    data.push(*c);
                }
                '>' => {
                    if count >= 2 {
                        let c_ult = data.pop();
                        let c_pen = data.pop();
                        match c_pen {
                            Some(']') => match c_ult {
                                Some(']') => {
                                    let cdsect = CDSect {
                                        start: start,
                                        text: data,
                                    };
                                    return Ok(cdsect);
                                }
                                Some(c) => {
                                    return Err(XmlError::BadChar(c));
                                }
                                None => {
                                    unreachable!(
                                        "hit unreachable condition when checking close delim for CDSect"
                                    );
                                }
                            },
                            Some(c) => {
                                return Err(XmlError::BadChar(c));
                            }
                            None => {
                                unreachable!(
                                    "hit unreachable condition when checking close delim for CDSect"
                                );
                            }
                        }
                    } else {
                        count = 0;
                        data.push(*c);
                    }
                }
                _ => {
                    count = 0;
                    data.push(*c);
                }
            }
        }
        Err(XmlError::TextEnd)
    } else {
        Err(XmlError::BadCDATAStart)
    }
}

fn parse_content_item(
    text: &[char],
    start: usize,
    recurdepth: usize,
) -> Result<ContentItem, XmlError> {
    if let Ok(reference) = parse_reference(text, start) {
        let item = ContentItem::Reference {
            start: start,
            reference: reference,
        };
        Ok(item)
    } else if let Ok(comment) = parse_comment(text, start) {
        let item = ContentItem::Comment(comment);
        Ok(item)
    } else if let Ok(pi) = parse_pi(text, start) {
        let item = ContentItem::ProcInstr(pi);
        Ok(item)
    } else if let Ok(chardata) = parse_chardata(text, start) {
        let item = ContentItem::CharData(chardata);
        Ok(item)
    } else if let Ok(cdsect) = parse_cdsect(text, start) {
        let item = ContentItem::CDSect(cdsect);
        Ok(item)
    } else if let Ok(elem) = parse_elem(text, start, recurdepth + 1) {
        let boxed_elem = Box::new(elem);
        let item = ContentItem::Elem(boxed_elem);
        Ok(item)
    } else {
        let err = XmlError::NoValidVariant;
        Err(err)
    }
}

fn parse_endtag(text: &[char], start: usize) -> Result<ETag, XmlError> {
    let c0 = *text.get(start).ok_or(XmlError::TextEnd)?;
    if c0 == '<' {
        let c1 = *text.get(start + 1).ok_or(XmlError::TextEnd)?;
        if c1 == '/' {
            let name = parse_name(text, start + 2)?;
            let pos = start + 2 + name.0.len();
            let closepos = match parse_ws(text, pos) {
                Ok(ws) => ws.get_endpos(),
                Err(_) => pos,
            };
            let c_last = *text.get(closepos).ok_or(XmlError::TextEnd)?;
            if c_last == '>' {
                let end = closepos + 1;
                let etag = ETag {
                    start: start,
                    end: end,
                    name: name,
                };
                Ok(etag)
            } else {
                Err(XmlError::BadChar(c_last))
            }
        } else {
            Err(XmlError::BadChar(c1))
        }
    } else {
        Err(XmlError::BadChar(c0))
    }
}

fn parse_attvalue(text: &[char], start: usize) -> Result<AttValue, XmlError> {
    let c0 = *text.get(start).ok_or(XmlError::TextEnd)?;
    let single_qoute = c0 == '\'';
    let mut items: Vec<AttValueItem> = Vec::new();
    let mut end_hit = false;
    let mut idx = start + 1;
    let mut current_item = String::new();
    while !end_hit {
        let c = *text.get(idx).ok_or(XmlError::TextEnd)?;
        if c == '\'' && single_qoute {
            end_hit = true;
            if current_item.len() > 0 {
                let item = AttValueItem::Text(current_item);
                items.push(item);
            }
            break;
        } else if c == '\"' && !single_qoute {
            end_hit = true;
            if current_item.len() > 0 {
                let item = AttValueItem::Text(current_item);
                items.push(item);
            }
            break;
        } else if c == '<' {
            let err = XmlError::BadChar(c);
            return Err(err);
        } else if c == '&' {
            if current_item.len() > 0 {
                let item = AttValueItem::Text(current_item);
                items.push(item);
                current_item = String::new();
            }
            let reference = parse_reference(text, idx)?;
            let item = AttValueItem::Reference(reference);
            let length = item.text_len();
            items.push(item);
            idx += length;
        } else {
            current_item.push(c);
            idx += 1;
        }
    }

    let attvalue = AttValue {
        start: start,
        items: items,
    };

    Ok(attvalue)
}

fn parse_reference(text: &[char], start: usize) -> Result<Reference, XmlError> {
    let c0 = *text.get(start).ok_or(XmlError::TextEnd)?;
    if c0 == '&' {
        let c1 = *text.get(start + 1).ok_or(XmlError::TextEnd)?;
        if c1 == '#' {
            let mut ref_text = String::new();
            let mut at_start = true;
            for c in &text[(start + 2)..] {
                if c == &';' {
                    break;
                } else {
                    match *c {
                        '0'..='9' | 'a'..='f' | 'A'..='F' => {
                            ref_text.push(*c);
                        }
                        'x' => {
                            if at_start {
                                ref_text.push(*c);
                            } else {
                                return Err(XmlError::BadChar(*c));
                            }
                        }
                        _ => {
                            return Err(XmlError::BadChar(*c));
                        }
                    };
                }
                at_start = false;
            }
            let reference = Reference::CharRef(ref_text);
            Ok(reference)
        } else {
            let name = parse_name(text, start + 1)?;
            let pos = start + 1 + name.0.len();
            let c_last = *text.get(pos).ok_or(XmlError::TextEnd)?;
            if c_last == ';' {
                let reference = Reference::EntityRef(name);
                Ok(reference)
            } else {
                Err(XmlError::BadChar(c_last))
            }
        }
    } else {
        Err(XmlError::BadChar(c0))
    }
}

pub struct Prolog {
    xml_decl: Option<XmlDecl>,
    doctype_decl: Option<DoctypeDecl>,
    miscs: Vec<Misc>,
}

pub struct XmlDecl {
    start: usize,
    end: usize,
    version: VersionInfo,
    encoding: Option<Encoding>,
    standalone: Option<SDDecl>,
}

struct VersionInfo {
    start: usize,
    end: usize,
    ver_num: f32,
}

struct Encoding {
    start: usize,
    end: usize,
    enc_name: String,
}

struct SDDecl {
    start: usize,
    end: usize,
    is_standalone: bool,
}

struct DoctypeDecl {
    start: usize,
    end: usize,
    name: Name,
    ext_id: Option<ExternalID>,
    int_subset: Option<IntSubset>,
}

#[derive(Debug)]
enum ExternalID {
    System {
        start: usize,
        end: usize,
        sys_lit: String,
    },
    Public {
        start: usize,
        end: usize,
        pub_lit: String,
        sys_lit: String,
    },
}

struct IntSubset {
    items: Vec<IntSubsetItem>,
}

enum IntSubsetItem {
    Blank(Ws),
    PEReference(PEReference),
    ElemDecl(ElemDecl),
    AttlistDecl(AttlistDecl),
    EntityDecl(EntityDecl),
    NotationDecl(NotationDecl),
    ProcInstr(ProcInstr),
    Comment(Comment),
}

struct PEReference(Name);

impl PEReference {
    fn textlen(&self) -> usize {
        self.0.0.len() + 2 // take delimiters into account
    }
}

struct ElemDecl;

struct AttlistDecl;

enum EntityDecl{}

struct NotationDecl;

pub enum Elem {
    Empty(EmptyElem),
    Full(FullElem),
}

pub struct EmptyElem {
    start: usize,
    end: usize,
    name: Name,
    attribs: Vec<Attribute>,
}

pub struct FullElem {
    start: STag,
    content: Option<Content>,
    end: ETag,
}

struct STag {
    start: usize,
    end: usize,
    name: Name,
    attribs: Vec<Attribute>,
}

struct ETag {
    start: usize,
    end: usize,
    name: Name,
}

pub struct Attribute {
    start: usize,
    end: usize,
    name: Name,
    value: AttValue,
}

struct AttValue {
    start: usize,
    items: Vec<AttValueItem>,
}

enum AttValueItem {
    Text(String),
    Reference(Reference),
}

impl AttValueItem {
    fn text_len(&self) -> usize {
        match &self {
            AttValueItem::Text(s) => s.len(),
            AttValueItem::Reference(reference) => reference.text_len(),
        }
    }
}

enum Reference {
    EntityRef(Name),
    CharRef(String),
}

impl Reference {
    fn text_len(&self) -> usize {
        match &self {
            Reference::EntityRef(name) => name.0.len() + 2,
            Reference::CharRef(s) => s.len() + 3,
        }
    }
}

struct Content {
    start: usize,
    items: Vec<ContentItem>,
}

enum ContentItem {
    Elem(Box<Elem>),
    Reference { start: usize, reference: Reference },
    ProcInstr(ProcInstr),
    Comment(Comment),
    CharData(CharData),
    CDSect(CDSect),
}

struct CDSect {
    start: usize,
    text: String,
}

struct CharData {
    start: usize,
    text: String,
}

pub enum Misc {
    Ws(Ws),
    Comment(Comment),
    ProcInstr(ProcInstr),
}

#[derive(PartialEq, Debug)]
pub struct Ws {
    start: usize,
    text: String,
}

pub struct Comment {
    start: usize,
    text: String,
}

pub struct ProcInstr {
    start: usize,
    target: PITarget,
    space: Option<Ws>,
    arg: Option<String>,
}

struct PITarget {
    name: Name,
}

struct Name(String);

struct EqHelper {
    start: usize,
    end: usize,
}
