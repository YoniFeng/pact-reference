//! Functions to support processing request/response bodies

use pact_matching::models::matchingrules::{MatchingRuleCategory, MatchingRule, RuleLogic};
use pact_matching::models::generators::{Generators, Generator, GeneratorCategory};
use serde_json::{Value, Map};
use pact_matching::models::json_utils::{json_to_string, json_to_num};
use pact_matching::models::{Request, OptionalBody, Response};
use maplit::*;
use hyper::header::Headers;
use formdata::{FormData, FilePart};
use std::path::Path;
use log::*;

const CONTENT_TYPE_HEADER: &str = "Content-Type";

/// Process an array with embedded matching rules and generators
pub fn process_array(
  array: &[Value],
  matching_rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &str,
  type_matcher: bool,
  skip_matchers: bool
) -> Value {
  Value::Array(array.iter().enumerate().map(|(index, val)| {
    let updated_path = if type_matcher {
      format!("{}[*]", path)
    } else {
      format!("{}[{}]", path, index)
    };
    match val {
      Value::Object(ref map) => process_object(map, matching_rules, generators, &updated_path, false, skip_matchers),
      Value::Array(ref array) => process_array(array, matching_rules, generators, &updated_path, false, skip_matchers),
      _ => val.clone()
    }
  }).collect())
}

/// Process an object (map) with embedded matching rules and generators
pub fn process_object(
  obj: &Map<String, Value>,
  matching_rules: &mut MatchingRuleCategory,
  generators: &mut Generators,
  path: &str,
  type_matcher: bool,
  skip_matchers: bool
) -> Value {
  if obj.contains_key("pact:matcher:type") {
    if !skip_matchers {
      let matching_rule = from_integration_json(obj);
      if let Some(rule) = &matching_rule {
        matching_rules.add_rule(&path.to_string(), rule.clone(), &RuleLogic::And);
      }
      if let Some(gen) = obj.get("pact:generator:type") {
        if let Some(generator) = Generator::from_map(&json_to_string(gen), obj) {
          generators.add_generator_with_subcategory(&GeneratorCategory::BODY, path, generator);
        }
      }
      let (value, skip_matchers) = if let Some(rule) = matching_rule {
        match rule {
          MatchingRule::ArrayContains(_) => (obj.get("variants"), true),
          _ => (obj.get("value"), false)
        }
      } else {
        (obj.get("value"), false)
      };
      match value {
        Some(val) => match val {
          Value::Object(ref map) => process_object(map, matching_rules, generators, path, true, skip_matchers),
          Value::Array(array) => process_array(array, matching_rules, generators, path, true, skip_matchers),
          _ => val.clone()
        },
        None => Value::Null
      }
    } else {
      match obj.get("value") {
        Some(val) => match val {
          Value::Object(ref map) => process_object(map, matching_rules, generators, path, false, skip_matchers),
          Value::Array(array) => process_array(array, matching_rules, generators, path, false, skip_matchers),
          _ => val.clone()
        },
        None => Value::Null
      }
    }
  } else {
    Value::Object(obj.iter()
      .filter(|(key, _)| !key.starts_with("pact:"))
      .map(|(key, val)| {
      let updated_path = if type_matcher {
        format!("{}.*", path)
      } else {
        format!("{}.{}", path, key)
      };
      (key.clone(), match val {
        Value::Object(ref map) => process_object(map, matching_rules, generators, &updated_path, false, skip_matchers),
        Value::Array(ref array) => process_array(array, matching_rules, generators, &updated_path, false, skip_matchers),
        _ => val.clone()
      })
    }).collect())
  }
}

/// Builds a `MatchingRule` from a `Value` struct used by language integrations
pub fn from_integration_json(m: &Map<String, Value>) -> Option<MatchingRule> {
  match m.get("pact:matcher:type") {
    Some(value) => {
      let val = json_to_string(value);
      match val.as_str() {
        "regex" => match m.get(&val) {
          Some(s) => Some(MatchingRule::Regex(json_to_string(s))),
          None => None
        },
        "equality" => Some(MatchingRule::Equality),
        "include" => match m.get("value") {
          Some(s) => Some(MatchingRule::Include(json_to_string(s))),
          None => None
        },
        "type" => match (json_to_num(m.get("min").cloned()), json_to_num(m.get("max").cloned())) {
          (Some(min), Some(max)) => Some(MatchingRule::MinMaxType(min, max)),
          (Some(min), None) => Some(MatchingRule::MinType(min)),
          (None, Some(max)) => Some(MatchingRule::MaxType(max)),
          _ => Some(MatchingRule::Type)
        },
        "number" => Some(MatchingRule::Number),
        "integer" => Some(MatchingRule::Integer),
        "decimal" => Some(MatchingRule::Decimal),
        "real" => Some(MatchingRule::Decimal),
        "min" => match json_to_num(m.get(&val).cloned()) {
          Some(min) => Some(MatchingRule::MinType(min)),
          None => None
        },
        "max" => match json_to_num(m.get(&val).cloned()) {
          Some(max) => Some(MatchingRule::MaxType(max)),
          None => None
        },
        "timestamp" => match m.get("format").or_else(|| m.get(&val)) {
          Some(s) => Some(MatchingRule::Timestamp(json_to_string(s))),
          None => None
        },
        "date" => match m.get("format").or_else(|| m.get(&val)) {
          Some(s) => Some(MatchingRule::Date(json_to_string(s))),
          None => None
        },
        "time" => match m.get("format").or_else(|| m.get(&val)) {
          Some(s) => Some(MatchingRule::Time(json_to_string(s))),
          None => None
        },
        "null" => Some(MatchingRule::Null),
        "contentType" => match m.get("value") {
          Some(s) => Some(MatchingRule::ContentType(json_to_string(s))),
          None => None
        }
        "arrayContains" => match m.get("variants") {
          Some(variants) => match variants {
            Value::Array(variants) => {
              let values = variants.iter().enumerate().map(|(index, variant)| {
                let mut category = MatchingRuleCategory::empty("body");
                let mut generators = Generators::default();
                match variant {
                  Value::Object(map) => {
                    process_object(map, &mut category, &mut generators, "$", false, false);
                  }
                  _ => warn!("arrayContains: JSON for variant {} is not correctly formed: {}", index, variant)
                }
                (index, category)
              }).collect();
              Some(MatchingRule::ArrayContains(values))
            }
            _ => None
          }
          None => None
        }
        _ => None
      }
    },
    _ => None
  }
}

/// Process a JSON body with embedded matching rules and generators
pub fn process_json(body: String, matching_rules: &mut MatchingRuleCategory, generators: &mut Generators) -> String {
  match serde_json::from_str(&body) {
    Ok(json) => match json {
      Value::Object(ref map) => process_object(map, matching_rules, generators, &"$".to_string(), false, false).to_string(),
      Value::Array(ref array) => process_array(array, matching_rules, generators, &"$".to_string(), false, false).to_string(),
      _ => body
    },
    Err(_) => body
  }
}

/// Process a JSON body with embedded matching rules and generators
pub fn process_json_value(body: &Value, matching_rules: &mut MatchingRuleCategory, generators: &mut Generators) -> String {
  match body {
    Value::Object(ref map) => process_object(map, matching_rules, generators, &"$".to_string(), false, false).to_string(),
    Value::Array(ref array) => process_array(array, matching_rules, generators, &"$".to_string(), false, false).to_string(),
    _ => body.to_string()
  }
}

/// Setup the request as a multipart form upload
pub fn request_multipart(request: &mut Request, boundary: &str, body: OptionalBody, content_type: &str, part_name: &str) {
  request.body = body;
  match request.headers {
    Some(ref mut headers) => {
      headers.insert(CONTENT_TYPE_HEADER.to_string(), vec![format!("multipart/form-data; boundary={}", boundary)]);
    },
    None => {
      request.headers = Some(hashmap! {
        CONTENT_TYPE_HEADER.to_string() => vec![format!("multipart/form-data; boundary={}", boundary)]
      });
    }
  };
  request.matching_rules.add_category("body")
    .add_rule(format!("$['{}']", part_name), MatchingRule::ContentType(content_type.into()), &RuleLogic::And);
  request.matching_rules.add_category("header")
    .add_rule("Content-Type", MatchingRule::Regex(r"multipart/form-data;(\s*charset=[^;]*;)?\s*boundary=.*".into()), &RuleLogic::And);
}

/// Setup the response as a multipart form upload
pub fn response_multipart(response: &mut Response, boundary: &str, body: OptionalBody, content_type: &str, part_name: &str) {
  response.body = body;
  match response.headers {
    Some(ref mut headers) => {
      headers.insert(CONTENT_TYPE_HEADER.to_string(), vec![format!("multipart/form-data; boundary={}", boundary)]);
    },
    None => {
      response.headers = Some(hashmap! {
        CONTENT_TYPE_HEADER.to_string() => vec![format!("multipart/form-data; boundary={}", boundary)]
      });
    }
  }
  response.matching_rules.add_category("body")
    .add_rule(format!("$['{}']", part_name), MatchingRule::ContentType(content_type.into()), &RuleLogic::And);
  response.matching_rules.add_category("header")
    .add_rule("Content-Type", MatchingRule::Regex(r"multipart/form-data;(\s*charset=[^;]*;)?\s*boundary=.*".into()), &RuleLogic::And);
}

/// Loads an example file as a MIME Multipart body
pub fn file_as_multipart_body(file: &str, part_name: &str, boundary: &str) -> Result<OptionalBody, String> {
  let headers = Headers::new();
  let formdata = FormData {
    fields: vec![],
    files: vec![(part_name.to_string(), FilePart::new(headers, Path::new(file)))]
  };
  let mut buffer: Vec<u8> = vec![];
  match formdata::write_formdata(&mut buffer, &boundary.as_bytes().to_vec(), &formdata) {
    Ok(_) => Ok(OptionalBody::Present(buffer.clone(), Some("multipart/form-data".into()))),
    Err(err) => {
      warn!("convert_ptr_to_mime_part_body: Failed to generate multipart body: {}", err);
      Err(format!("convert_ptr_to_mime_part_body: Failed to generate multipart body: {}", err))
    }
  }
}
