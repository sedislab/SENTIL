// A small recursive-descent JSON reader, just enough to load the shared oracle and
// benchmark fixtures. The C++ standard library has no JSON, and the binding stays
// dependency-free, so the tests carry this rather than pulling one in. It parses
// the subset the fixtures use: objects, arrays, strings, numbers, booleans, null.
#ifndef SENTIL_TEST_JSON_HPP
#define SENTIL_TEST_JSON_HPP

#include <cstddef>
#include <map>
#include <stdexcept>
#include <string>
#include <vector>

namespace testjson {

struct Value {
    enum class Type { Null, Bool, Number, String, Array, Object };
    Type type = Type::Null;
    bool boolean = false;
    double number = 0.0;
    std::string text;
    std::vector<Value> items;
    std::map<std::string, Value> members;

    const Value& operator[](const std::string& key) const {
        auto it = members.find(key);
        if (it == members.end()) {
            throw std::runtime_error("missing JSON key: " + key);
        }
        return it->second;
    }
    const Value& operator[](std::size_t index) const { return items.at(index); }
    std::size_t size() const { return items.size(); }
};

class Parser {
public:
    explicit Parser(const std::string& source) : src_(source) {}

    Value parse() {
        Value value = parse_value();
        skip_ws();
        if (pos_ != src_.size()) {
            throw std::runtime_error("trailing characters in JSON");
        }
        return value;
    }

private:
    const std::string& src_;
    std::size_t pos_ = 0;

    void skip_ws() {
        while (pos_ < src_.size()) {
            char c = src_[pos_];
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
                ++pos_;
            } else {
                break;
            }
        }
    }

    char peek() {
        skip_ws();
        if (pos_ >= src_.size()) {
            throw std::runtime_error("unexpected end of JSON");
        }
        return src_[pos_];
    }

    void expect(char c) {
        if (peek() != c) {
            throw std::runtime_error(std::string("expected '") + c + "' in JSON");
        }
        ++pos_;
    }

    Value parse_value() {
        char c = peek();
        switch (c) {
            case '{':
                return parse_object();
            case '[':
                return parse_array();
            case '"':
                return parse_string();
            case 't':
            case 'f':
                return parse_bool();
            case 'n':
                return parse_null();
            default:
                return parse_number();
        }
    }

    Value parse_object() {
        Value value;
        value.type = Value::Type::Object;
        expect('{');
        if (peek() == '}') {
            ++pos_;
            return value;
        }
        while (true) {
            Value key = parse_string();
            expect(':');
            value.members.emplace(key.text, parse_value());
            char c = peek();
            ++pos_;
            if (c == '}') {
                break;
            }
            if (c != ',') {
                throw std::runtime_error("expected ',' or '}' in JSON object");
            }
        }
        return value;
    }

    Value parse_array() {
        Value value;
        value.type = Value::Type::Array;
        expect('[');
        if (peek() == ']') {
            ++pos_;
            return value;
        }
        while (true) {
            value.items.push_back(parse_value());
            char c = peek();
            ++pos_;
            if (c == ']') {
                break;
            }
            if (c != ',') {
                throw std::runtime_error("expected ',' or ']' in JSON array");
            }
        }
        return value;
    }

    Value parse_string() {
        Value value;
        value.type = Value::Type::String;
        expect('"');
        while (pos_ < src_.size()) {
            char c = src_[pos_++];
            if (c == '"') {
                return value;
            }
            if (c == '\\') {
                char esc = src_[pos_++];
                switch (esc) {
                    case '"': value.text.push_back('"'); break;
                    case '\\': value.text.push_back('\\'); break;
                    case '/': value.text.push_back('/'); break;
                    case 'b': value.text.push_back('\b'); break;
                    case 'f': value.text.push_back('\f'); break;
                    case 'n': value.text.push_back('\n'); break;
                    case 'r': value.text.push_back('\r'); break;
                    case 't': value.text.push_back('\t'); break;
                    default: throw std::runtime_error("unsupported JSON escape");
                }
            } else {
                value.text.push_back(c);
            }
        }
        throw std::runtime_error("unterminated JSON string");
    }

    Value parse_bool() {
        Value value;
        value.type = Value::Type::Bool;
        if (src_.compare(pos_, 4, "true") == 0) {
            value.boolean = true;
            pos_ += 4;
        } else if (src_.compare(pos_, 5, "false") == 0) {
            value.boolean = false;
            pos_ += 5;
        } else {
            throw std::runtime_error("invalid JSON literal");
        }
        return value;
    }

    Value parse_null() {
        if (src_.compare(pos_, 4, "null") != 0) {
            throw std::runtime_error("invalid JSON literal");
        }
        pos_ += 4;
        Value value;
        value.type = Value::Type::Null;
        return value;
    }

    Value parse_number() {
        std::size_t start = pos_;
        while (pos_ < src_.size()) {
            char c = src_[pos_];
            if ((c >= '0' && c <= '9') || c == '-' || c == '+' || c == '.' || c == 'e' ||
                c == 'E') {
                ++pos_;
            } else {
                break;
            }
        }
        Value value;
        value.type = Value::Type::Number;
        value.number = std::stod(src_.substr(start, pos_ - start));
        return value;
    }
};

inline Value parse(const std::string& source) { return Parser(source).parse(); }

}  // namespace testjson

#endif  // SENTIL_TEST_JSON_HPP