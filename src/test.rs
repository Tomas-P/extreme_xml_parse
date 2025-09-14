use super::*;

#[test]
fn recog_comment() {
    let text = "<!--This is a valid comment-->";
    let chars: Vec<char> = text.chars().collect();
    let cparse = parse_comment(&chars, 0);
    match cparse {
        Ok(comment) => {
            let c_text = comment.text;
            assert_eq!(c_text, "This is a valid comment");
        }
        Err(e) => {
            println!("Comment parsing failed: {:?}", e);
            assert!(false);
        }
    }
}

#[test]
fn reject_invalid_comment() {
    let text = "<!-- This comment contains an illegal -- substring -->";
    let chars: Vec<char> = text.chars().collect();
    let cparse = parse_comment(&chars, 0);
    match cparse {
        Ok(_comment) => {
            assert!(false, "Failed to reject invalid comment");
        }
        Err(e) => match e {
            XmlError::IllegalSubstr => (),
            _ => {
                assert!(false, "Expected error variant is IllegalSubstr: {:?}", e);
            }
        },
    }
}

#[test]
fn take_pi_noarg() {
    let text = "<?NoArgumentPI?>";
    let chars: Vec<char> = text.chars().collect();
    let piparse = parse_pi(&chars, 0);
    match piparse {
        Ok(pi) => {
            assert_eq!(pi.target.name.0, "NoArgumentPI");
            assert_eq!(pi.space, None);
            assert_eq!(pi.arg, None);
        }
        Err(e) => {
            assert!(
                false,
                "Expected to parse no-argument PI, got error: {:?}",
                e
            );
        }
    }
}

#[test]
fn take_pi_witharg() {
    let text = "<?PIname argtext1 argtext1 ?>";
    let chars: Vec<char> = text.chars().collect();
    let pi_parse = parse_pi(&chars, 0);
    match pi_parse {
        Ok(pi) => {
            assert_eq!(pi.target.name.0, "PIname");
            match pi.space {
                Some(ws) => {
                    assert_eq!(ws.text, " ");
                }
                None => assert!(false, "expected space present"),
            };
            match pi.arg {
                Some(s) => {
                    assert_eq!(s, "argtext1 argtext1 ");
                }
                None => assert!(false, "expected argument"),
            };
        }
        Err(e) => {
            assert!(false, "Expected to parse PI, got error: {:?}", e);
        }
    }
}

#[test]
fn reject_xmlpi() {
    let text = "<?xml?>";
    let chars: Vec<char> = text.chars().collect();
    let pi_parse = parse_pi(&chars, 0);
    match pi_parse {
        Ok(_) => assert!(false, "should have rejected name XML in PI context"),
        Err(e) => match e {
            XmlError::ReservedNameXml => (),
            _ => assert!(false, "expected error ReservedNameXml, got {:?}", e),
        },
    }
}

#[test]
fn correct_endpos_pi1() {
    let text = "<?target?>";
    let chars: Vec<char> = text.chars().collect();
    let pi_parse = parse_pi(&chars, 0).expect("Failed to parse example");
    assert_eq!(pi_parse.get_endpos(), text.len());
}

#[test]
fn recognize_ws() {
    let text = " \n\t\r \n \t \r";
    let chars: Vec<char> = text.chars().collect();
    let ws_parse = parse_ws(&chars, 0);
    match ws_parse {
        Ok(_) => (),
        Err(e) => assert!(false, "expected to parse whitespace, got error: {:?}", e),
    }
}

#[test]
fn recognize_misc() {
    let text1 = "    ";
    let chars1: Vec<char> = text1.chars().collect();
    let misc1 = parse_misc(&chars1, 0);
    match misc1 {
        Ok(_) => (),
        Err(e) => {
            assert!(
                false,
                "expected to parse whitespace as misc, error: {:?}",
                e
            )
        }
    };
    let text2 = "<?pithing?>";
    let chars2: Vec<char> = text2.chars().collect();
    let misc2 = parse_misc(&chars2, 0);
    match misc2 {
        Ok(_) => (),
        Err(e) => {
            assert!(false, "expected to parse PI as misc, error: {:?}", e);
        }
    };
    let text3 = "<!-- Comment text -->";
    let chars3: Vec<char> = text3.chars().collect();
    let misc3 = parse_misc(&chars3, 0);
    match misc3 {
        Ok(_) => (),
        Err(e) => {
            assert!(false, "expected to parse comment as misc, error: {:?}", e);
        }
    };
}

#[test]
fn recognize_tail() {
    let text = "  <!-- this is a comment --> \t <?parse_instruct argument includes this?> \n  ";
    let chars: Vec<char> = text.chars().collect();
    let tail_parse = parse_tail(&chars, 0);
    match tail_parse {
        Ok(_) => (),
        Err(e) => {
            assert!(false, "should be valid parse, instead error: {:?}", e)
        }
    };
}

#[test]
fn recognize_empty_noarg() {
    let text = "<EmptyTag/>";
    let chars: Vec<char> = text.chars().collect();
    let empty_parse = parse_empty_elem(&chars, 0);
    match empty_parse {
        Ok(empty) => {
            assert_eq!(empty.name.0, "EmptyTag");
        }
        Err(e) => {
            assert!(false, "should be valid parse, instead get error: {:?}", e);
        }
    };
}

#[test]
fn recognize_empty_trailws() {
    let text = "<EmptyTrail    />";
    let chars: Vec<char> = text.chars().collect();
    let empty_parse = parse_empty_elem(&chars, 0);
    match empty_parse {
        Ok(empty) => {
            assert_eq!(empty.get_endpos(), chars.len())
        }
        Err(e) => assert!(false, "should be valid parse, instead got error: {:?}", e),
    }
}

#[test]
fn recognize_attval() {
    let text = "'thing text'";
    let chars: Vec<char> = text.chars().collect();
    let attval_parse = parse_attvalue(&chars, 0);
    match attval_parse {
        Ok(attval) => {
            let v0 = &attval.items[0];
            match v0 {
                AttValueItem::Text(s) => assert_eq!(s, "thing text"),
                _ => assert!(false, "did not expect to recognize reference"),
            };
            assert_eq!(attval.get_endpos(), chars.len());
        }
        Err(e) => assert!(false, "should be valid parse, instead got error: {:?}", e),
    }
}

#[test]
fn recognize_attribute() {
    let text = "AttribName = 'value text'";
    let chars: Vec<char> = text.chars().collect();
    let attrib_parse = parse_attribute(&chars, 0);
    match attrib_parse {
        Ok(attrib) => {
            assert_eq!(attrib.get_endpos(), chars.len());
            assert_eq!(attrib.name.0, "AttribName");
        }
        Err(e) => assert!(false, "should be valid parse, instead got error: {:?}", e),
    }
}

#[test]
fn recognize_reference() {
    let text = "&SomeItem;";
    let chars: Vec<char> = text.chars().collect();
    let ref_parse = parse_reference(&chars, 0);
    match ref_parse {
        Ok(_reference) => (),
        Err(e) => assert!(false, "should be valid parse, instead got error: {:?}", e),
    }
}

#[test]
fn recognize_empty_1arg() {
    let text = "<EmptyTag Attrib1 = \"Value 1\" />";
    let chars: Vec<char> = text.chars().collect();
    let empty_parse = parse_empty_elem(&chars, 0);
    match empty_parse {
        Ok(empty) => assert_eq!(empty.attribs.len(), 1),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_empty_ref_2arg() {
    let text = "<EmptyTag attrib1 = \"Value 1\" attrib2 = \"&RefItem;\" />";
    let chars: Vec<char> = text.chars().collect();
    let empty_parse = parse_empty_elem(&chars, 0);
    match empty_parse {
        Ok(empty) => assert_eq!(empty.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_end_tag() {
    let text = "</EndTag>";
    let chars: Vec<char> = text.chars().collect();
    let etag_parse = parse_endtag(&chars, 0);
    match etag_parse {
        Ok(etag) => assert_eq!(etag.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead got: {:?}", e),
    }
}

#[test]
fn recognize_end_tag_trailws() {
    let text = "</TagSpace     >";
    let chars: Vec<char> = text.chars().collect();
    let etag_parse = parse_endtag(&chars, 0);
    match etag_parse {
        Ok(etag) => assert_eq!(etag.name.0, "TagSpace"),
        Err(e) => assert!(false, "should be valid parse, instead got: {:?}", e),
    }
}

#[test]
fn reject_bad_endtag() {
    let text = "</EndTag stuff that is not supposed to be here>";
    let chars: Vec<char> = text.chars().collect();
    let etag_parse = parse_endtag(&chars, 0);
    match etag_parse {
        Ok(_etag) => assert!(false, "This should be rejected"),
        Err(_e) => (),
    }
}

#[test]
fn recognize_starttag() {
    let text = "<StartTag>";
    let chars: Vec<char> = text.chars().collect();
    let stag_parse = parse_starttag(&chars, 0);
    match stag_parse {
        Ok(s_tag) => assert_eq!(s_tag.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid, instead: {:?}", e),
    }
}

#[test]
fn recognize_starttag_attribs() {
    let text = "<StartTag Attrib1=\"Value 1\" Attrib2=\'&RefValue2;\' >";
    let chars: Vec<char> = text.chars().collect();
    let stag_parse = parse_starttag(&chars, 0);
    match stag_parse {
        Ok(s_tag) => assert_eq!(s_tag.attribs.len(), 2),
        Err(e) => assert!(false, "should be valid, instead: {:?}", e),
    }
}

#[test]
fn recognize_data() {
    let text ="<TagName> data goes here </TagName>";
    let chars :Vec<char> = text.chars().collect();
    let elem_parse = parse_elem(&chars, 0, 0);
    match elem_parse {
        Ok(elem) => assert_eq!(elem.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_cdsect() {
    let text = "<![CDATA[ this is a CDATA section ]]>";
    let chars :Vec<char> = text.chars().collect();
    let cdata_parse = parse_cdsect(&chars, 0);
    match cdata_parse {
        Ok(cdsect) => assert_eq!(cdsect.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_nested_elems() {
    let text = "<outer> 
    stuff 
    <?more stuff?> 
    <inner>
    <!-- some other stuff -->
    </inner>
    <![CDATA[ even more stuff ]]>

    </outer>";

    let chars :Vec<char> = text.chars().collect();

    let elem_parse = parse_elem(&chars, 0, 0);
    match elem_parse {
        Ok(_elem) => (),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_version() {
    let text = "   version    = \t  \"1.0\"";
    let chars :Vec<char> = text.chars().collect();
    let ver_parse = parse_version(&chars, 0);
    match ver_parse {
        Ok(ver) => assert_eq!(ver.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_encoding() {
    let text = "  encoding = 'utf-8'";
    let chars :Vec<char> = text.chars().collect();
    let enc_parse = parse_encoding(&chars, 0);
    match enc_parse {
        Ok(enc) => assert_eq!(enc.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_standalone() {
    let text = "   standalone =  \"yes\"";
    let chars :Vec<char> = text.chars().collect();
    let stand_parse = parse_standalone(&chars, 0);
    match stand_parse {
        Ok(stand) => assert_eq!(stand.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

#[test]
fn recognize_xmldecl_version() {
    let text = "<?xml version = \'1.0\' ?>";
    let chars :Vec<char> = text.chars().collect();
    let xdecl_parse = parse_xmldecl(&chars, 0);
    match xdecl_parse {
        Ok(xdecl) => assert_eq!(xdecl.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead :{:?}", e),
    }
}

#[test]
fn recognize_xmldecl() {
    let text = "<?xml version = \'1.0\' encoding = \'utf-8\' standalone = \'yes\' ?>";
    let chars :Vec<char> = text.chars().collect();
    let xdecl_parse = parse_xmldecl(&chars, 0);
    match xdecl_parse {
        Ok(xdecl) => assert_eq!(xdecl.get_endpos(), chars.len()),
        Err(e) => assert!(false, "should be valid parse, instead: {:?}", e),
    }
}

