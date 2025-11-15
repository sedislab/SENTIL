package io.github.sedislab.sentil;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** A small JSON reader for the benchmark fixtures. */
final class Json {
    enum Kind {
        OBJECT, ARRAY, STRING, NUMBER, BOOL, NULL
    }

    final Kind kind;
    String text = "";
    boolean bool;
    final Map<String, Json> members = new LinkedHashMap<>();
    final List<Json> items = new ArrayList<>();

    private Json(Kind kind) {
        this.kind = kind;
    }

    Json get(String key) {
        return members.get(key);
    }

    Json get(int index) {
        return items.get(index);
    }

    int size() {
        return items.size();
    }

    String str() {
        return text;
    }

    int asInt() {
        return (int) Double.parseDouble(text);
    }

    static Json parse(String source) {
        Parser parser = new Parser(source);
        Json value = parser.value();
        parser.skipWhitespace();
        if (!parser.atEnd()) {
            throw new IllegalArgumentException("trailing content at " + parser.position());
        }
        return value;
    }

    private static final class Parser {
        private final String src;
        private int pos;

        Parser(String src) {
            this.src = src;
        }

        int position() {
            return pos;
        }

        boolean atEnd() {
            return pos >= src.length();
        }

        void skipWhitespace() {
            while (pos < src.length() && Character.isWhitespace(src.charAt(pos))) {
                pos++;
            }
        }

        Json value() {
            skipWhitespace();
            char c = src.charAt(pos);
            switch (c) {
                case '{':
                    return object();
                case '[':
                    return array();
                case '"':
                    return string();
                case 't':
                case 'f':
                    return bool();
                case 'n':
                    expect("null");
                    return new Json(Kind.NULL);
                default:
                    return number();
            }
        }

        private Json object() {
            Json node = new Json(Kind.OBJECT);
            pos++;
            skipWhitespace();
            if (src.charAt(pos) == '}') {
                pos++;
                return node;
            }
            while (true) {
                skipWhitespace();
                String key = string().text;
                skipWhitespace();
                expectChar(':');
                node.members.put(key, value());
                skipWhitespace();
                char c = src.charAt(pos++);
                if (c == '}') {
                    return node;
                }
                if (c != ',') {
                    throw new IllegalArgumentException("expected , or } at " + (pos - 1));
                }
            }
        }

        private Json array() {
            Json node = new Json(Kind.ARRAY);
            pos++;
            skipWhitespace();
            if (src.charAt(pos) == ']') {
                pos++;
                return node;
            }
            while (true) {
                node.items.add(value());
                skipWhitespace();
                char c = src.charAt(pos++);
                if (c == ']') {
                    return node;
                }
                if (c != ',') {
                    throw new IllegalArgumentException("expected , or ] at " + (pos - 1));
                }
            }
        }

        private Json string() {
            Json node = new Json(Kind.STRING);
            expectChar('"');
            StringBuilder out = new StringBuilder();
            while (true) {
                char c = src.charAt(pos++);
                if (c == '"') {
                    break;
                }
                if (c == '\\') {
                    char esc = src.charAt(pos++);
                    switch (esc) {
                        case '"': out.append('"'); break;
                        case '\\': out.append('\\'); break;
                        case '/': out.append('/'); break;
                        case 'n': out.append('\n'); break;
                        case 't': out.append('\t'); break;
                        case 'r': out.append('\r'); break;
                        case 'b': out.append('\b'); break;
                        case 'f': out.append('\f'); break;
                        case 'u':
                            out.append((char) Integer.parseInt(src.substring(pos, pos + 4), 16));
                            pos += 4;
                            break;
                        default:
                            throw new IllegalArgumentException("bad escape at " + (pos - 1));
                    }
                } else {
                    out.append(c);
                }
            }
            node.text = out.toString();
            return node;
        }

        private Json number() {
            int start = pos;
            while (pos < src.length() && "+-0123456789.eE".indexOf(src.charAt(pos)) >= 0) {
                pos++;
            }
            Json node = new Json(Kind.NUMBER);
            node.text = src.substring(start, pos);
            return node;
        }

        private Json bool() {
            Json node = new Json(Kind.BOOL);
            if (src.charAt(pos) == 't') {
                expect("true");
                node.bool = true;
            } else {
                expect("false");
                node.bool = false;
            }
            return node;
        }

        private void expect(String literal) {
            if (!src.startsWith(literal, pos)) {
                throw new IllegalArgumentException("expected " + literal + " at " + pos);
            }
            pos += literal.length();
        }

        private void expectChar(char c) {
            if (src.charAt(pos) != c) {
                throw new IllegalArgumentException("expected " + c + " at " + pos);
            }
            pos++;
        }
    }
}