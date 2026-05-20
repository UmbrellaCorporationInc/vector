import { EditorSelection, EditorState } from "@codemirror/state";
import {
  autocompletion,
  closeCompletion,
  completionStatus,
  insertCompletionText,
  pickedCompletion,
  startCompletion,
} from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { Decoration, EditorView, keymap, ViewPlugin } from "@codemirror/view";

(function () {
  var FILE_SUGGESTIONS_REQUEST_TYPE = "vector.chatInput.requestSuggestions";
  var MIN_EDITOR_HEIGHT_PX = 64;
  var MAX_EDITOR_HEIGHT_PX = 320;
  var requestIdCounter = 0;
  var chatInputTheme = EditorView.theme({
    "&": {
      width: "100%",
      boxSizing: "border-box",
      position: "relative",
      background: "var(--vscode-input-background, rgba(255,255,255,0.06))",
      color: "var(--vscode-input-foreground, var(--vscode-editor-foreground, #cdd6f4))",
      border: "1px solid var(--vscode-input-border, rgba(255,255,255,0.15))",
      borderRadius: "4px",
      fontFamily: "var(--vscode-editor-font-family, sans-serif)",
      fontSize: "inherit",
      lineHeight: "1.6",
      outline: "none",
    },
    "&.cm-focused": {
      borderColor: "var(--vscode-focusBorder, rgba(255,255,255,0.35))",
      boxShadow:
        "0 0 0 1px color-mix(in srgb, var(--vscode-focusBorder, rgba(255,255,255,0.35)) 55%, transparent)",
    },
    ".cm-scroller": {
      overflowY: "auto",
      maxHeight: "20rem",
      fontFamily: "inherit",
    },
    ".cm-content": {
      minHeight: "4rem",
      padding: "0.35rem 0.5rem",
      whiteSpace: "pre-wrap",
      wordBreak: "break-word",
      caretColor: "var(--vscode-input-foreground, var(--vscode-editor-foreground, #cdd6f4))",
    },
    ".cm-line": {
      padding: "0",
    },
    "&.cm-focused .cm-cursor, & .cm-dropCursor": {
      borderLeftColor:
        "var(--vscode-input-foreground, var(--vscode-editor-foreground, #cdd6f4))",
    },
  });

  function detectMentionQuery(text, cursorPos) {
    if (cursorPos <= 0) return null;
    var before = text.slice(0, cursorPos);
    var match = /(?:^|[\s\n])@([^\s@]*)$/.exec(before);
    if (!match) return null;
    var query = match[1] || "";
    var atIndex = before.lastIndexOf("@");
    return { query: query, start: atIndex, end: cursorPos };
  }

  function insertMentionText(text, mentionQuery, suggestion) {
    var token = "@" + suggestion.path;
    var newText = text.slice(0, mentionQuery.start) + token + text.slice(mentionQuery.end);
    return { text: newText, cursorPos: mentionQuery.start + token.length };
  }

  function classifyLine(line) {
    if (/^#{1,6}\s/.test(line)) return "heading";
    if (/^(?:[-*+]|\d+\.)\s/.test(line)) return "list-item";
    if (/^```/.test(line)) return "fenced-code";
    return null;
  }

  function tokenizeInline(text) {
    var re = /(\*\*[^*\n]+\*\*|\*(?!\s)[^*\n]+(?<!\s)\*|`[^`\n]+`)/g;
    var tokens = [];
    var lastIndex = 0;
    var match;
    while ((match = re.exec(text)) !== null) {
      if (match.index > lastIndex) {
        tokens.push({ from: lastIndex, to: match.index, type: "plain" });
      }
      var raw = match[0];
      var type = raw.indexOf("**") === 0 ? "strong" : raw.indexOf("*") === 0 ? "em" : "code";
      tokens.push({ from: match.index, to: re.lastIndex, type: type });
      lastIndex = re.lastIndex;
    }
    if (lastIndex < text.length) {
      tokens.push({ from: lastIndex, to: text.length, type: "plain" });
    }
    return tokens;
  }

  function findMentionRanges(text, mentions) {
    var ranges = [];
    mentions.forEach(function (mention) {
      var token = "@" + mention.path;
      var searchFrom = 0;
      while (searchFrom < text.length) {
        var index = text.indexOf(token, searchFrom);
        if (index === -1) break;
        ranges.push({ mention: mention, from: index, to: index + token.length });
        searchFrom = index + token.length;
      }
    });
    ranges.sort(function (left, right) {
      return left.from - right.from || left.to - right.to;
    });
    return ranges;
  }

  function reconcileMentions(text, mentions) {
    var seen = new Set();
    return findMentionRanges(text, mentions).reduce(function (result, range) {
      var key = range.mention.type + ":" + range.mention.path;
      if (seen.has(key)) {
        return result;
      }
      seen.add(key);
      result.push(range.mention);
      return result;
    }, []);
  }

  function findMentionRangeAtCursor(text, mentions, cursorPos, direction) {
    var ranges = findMentionRanges(text, mentions);
    for (var i = 0; i < ranges.length; i += 1) {
      var range = ranges[i];
      if (direction === "backward" && cursorPos === range.to) {
        return range;
      }
      if (direction === "forward" && cursorPos === range.from) {
        return range;
      }
    }
    return null;
  }

  function createMarkdownMentionDecorations(getMentions) {
    var mentionMark = function (mention) {
      return Decoration.mark({
        class: "vector-chat-mention",
        attributes: {
          "data-mention-path": mention.path,
          "data-mention-label": mention.label,
          title: mention.path,
        },
      });
    };

    var inlineMark = function (className) {
      return Decoration.mark({ class: className });
    };

    var lineMarks = {
      heading: Decoration.line({ attributes: { class: "vc-md-heading" } }),
      "list-item": Decoration.line({ attributes: { class: "vc-md-list-item" } }),
      "fenced-code": Decoration.line({ attributes: { class: "vc-md-fenced-code" } }),
    };

    return ViewPlugin.fromClass(
      class {
        constructor(view) {
          this.decorations = this.build(view);
        }

        update(update) {
          if (update.docChanged || update.viewportChanged || update.selectionSet) {
            this.decorations = this.build(update.view);
          }
        }

        build(view) {
          var builder = [];
          var doc = view.state.doc;
          for (var lineNumber = 1; lineNumber <= doc.lines; lineNumber += 1) {
            var line = doc.line(lineNumber);
            var lineClass = classifyLine(line.text);
            if (lineClass && lineMarks[lineClass]) {
              builder.push(lineMarks[lineClass].range(line.from));
            }
            tokenizeInline(line.text).forEach(function (token) {
              if (token.type === "plain" || token.from === token.to) {
                return;
              }
              builder.push(
                inlineMark("vc-md-" + token.type).range(
                  line.from + token.from,
                  line.from + token.to,
                ),
              );
            });
          }

          var text = doc.toString();
          findMentionRanges(text, getMentions()).forEach(function (range) {
            builder.push(mentionMark(range.mention).range(range.from, range.to));
          });

          return Decoration.set(builder, true);
        }
      },
      {
        decorations: function (value) {
          return value.decorations;
        },
      },
    );
  }

  function createChatInputRuntime(options) {
    var vscode = options.vscode;
    var editorsByHost = new Map();
    var pendingRequests = new Map();
    var nonceMeta = document.querySelector('meta[name="vector-csp-nonce"]');
    var cspNonce =
      nonceMeta instanceof HTMLMetaElement ? nonceMeta.getAttribute("content") || "" : "";

    function scheduleEditorLayout(instance) {
      if (!instance.view) {
        return;
      }
      instance.view.requestMeasure({
        key: instance.layoutMeasureKey,
        read: function (view) {
          var contentStyle = window.getComputedStyle(view.contentDOM);
          var paddingTop = parseFloat(contentStyle.paddingTop || "0");
          var paddingBottom = parseFloat(contentStyle.paddingBottom || "0");
          var contentHeight = view.contentHeight + paddingTop + paddingBottom;
          return {
            boundedHeight: Math.min(
              Math.max(contentHeight, MIN_EDITOR_HEIGHT_PX),
              MAX_EDITOR_HEIGHT_PX,
            ),
            shouldScroll: contentHeight > MAX_EDITOR_HEIGHT_PX,
          };
        },
        write: function (measurement) {
          if (!instance.view) {
            return;
          }
          instance.view.scrollDOM.style.height = measurement.boundedHeight + "px";
          instance.view.scrollDOM.style.overflowY = measurement.shouldScroll ? "auto" : "hidden";
          instance.hostEl.style.minHeight = measurement.boundedHeight + "px";
        },
      });
    }

    function observeEditorSize(instance) {
      if (typeof ResizeObserver !== "function") {
        return;
      }
      instance.resizeObserver = new ResizeObserver(function () {
        scheduleEditorLayout(instance);
      });
      instance.resizeObserver.observe(instance.mountEl);
    }

    function completePendingRequest(requestId, suggestions) {
      var pending = pendingRequests.get(requestId);
      if (!pending) {
        return;
      }
      clearTimeout(pending.timeout);
      pendingRequests.delete(requestId);
      pending.resolve(Array.isArray(suggestions) ? suggestions : []);
    }

    function requestSuggestions(query, context) {
      return new Promise(function (resolve) {
        var requestId = "req-" + (++requestIdCounter);
        var timeout = setTimeout(function () {
          pendingRequests.delete(requestId);
          resolve([]);
        }, 3000);

        pendingRequests.set(requestId, {
          resolve: resolve,
          timeout: timeout,
        });

        if (context && typeof context.addEventListener === "function") {
          context.addEventListener(
            "abort",
            function () {
              if (!pendingRequests.has(requestId)) {
                return;
              }
              clearTimeout(timeout);
              pendingRequests.delete(requestId);
              resolve([]);
            },
            { onDocChange: true },
          );
        }

        vscode.postMessage({
          type: FILE_SUGGESTIONS_REQUEST_TYPE,
          requestId: requestId,
          query: query,
        });
      });
    }

    function syncMentions(instance) {
      instance.mentions = reconcileMentions(instance.view.state.doc.toString(), instance.mentions);
    }

    function buildCompletionOptions(instance, mentionQuery, suggestions) {
      return suggestions.map(function (suggestion) {
        return {
          label: suggestion.label,
          detail: suggestion.path,
          type: "file",
          boost: 99,
          section: "Workspace Files",
          apply: function (view, completion, from, to) {
            var insertion = insertMentionText(view.state.doc.toString(), mentionQuery, suggestion);
            var transaction = insertCompletionText(
              view.state,
              insertion.text.slice(mentionQuery.start, insertion.cursorPos),
              from,
              to,
            );
            transaction.annotations = pickedCompletion.of(completion);
            view.dispatch(transaction);
            view.dispatch({
              selection: EditorSelection.cursor(insertion.cursorPos),
            });
            if (!instance.mentions.some(function (mention) { return mention.path === suggestion.path; })) {
              instance.mentions.push({
                type: "file",
                label: suggestion.label,
                path: suggestion.path,
              });
            }
            syncMentions(instance);
            view.focus();
          },
        };
      });
    }

    function createMentionCompletionSource(instance) {
      return function (context) {
        var selection = context.state.selection.main;
        if (!selection.empty || context.pos !== selection.head) {
          return null;
        }

        var text = context.state.doc.toString();
        var mentionQuery = detectMentionQuery(text, context.pos);
        if (!mentionQuery) {
          return null;
        }

        return requestSuggestions(mentionQuery.query, context).then(function (suggestions) {
          if (context.aborted || !Array.isArray(suggestions) || suggestions.length === 0) {
            return null;
          }
          return {
            from: mentionQuery.start,
            to: mentionQuery.end,
            options: buildCompletionOptions(instance, mentionQuery, suggestions),
            filter: false,
          };
        });
      };
    }

    function maybeStartMentionCompletion(instance) {
      var cursorPos = instance.view.state.selection.main.head;
      var selection = instance.view.state.selection.main;
      if (!selection.empty) {
        closeCompletion(instance.view);
        return;
      }
      var text = instance.view.state.doc.toString();
      var mentionQuery = detectMentionQuery(text, cursorPos);
      if (!mentionQuery) {
        closeCompletion(instance.view);
        return;
      }
      if (completionStatus(instance.view.state) === null) {
        startCompletion(instance.view);
      }
    }

    function createEditor(hostEl, mountEl) {
      var labelId = hostEl.dataset.chatInputLabelId || "";
      var accessibleName = hostEl.dataset.chatInputName || "Prompt";
      var name = hostEl.dataset.chatInputName || "";
      var instance = {
        hostEl: hostEl,
        mountEl: mountEl,
        labelId: labelId,
        layoutMeasureKey: {},
        name: name,
        mentions: [],
        resizeObserver: null,
        view: null,
      };

      var updateListener = EditorView.updateListener.of(function (update) {
        if (update.docChanged || update.geometryChanged || update.viewportChanged) {
          scheduleEditorLayout(instance);
        }
        if (update.docChanged || update.selectionSet) {
          syncMentions(instance);
          maybeStartMentionCompletion(instance);
        }
      });

      var domHandlers = EditorView.domEventHandlers({
        keydown: function (_event, view) {
          var event = _event;
          var selection = view.state.selection.main;
          var text = view.state.doc.toString();
          if (selection.empty && event.key === "Backspace") {
            var backwardRange = findMentionRangeAtCursor(
              text,
              instance.mentions,
              selection.head,
              "backward",
            );
            if (backwardRange) {
              event.preventDefault();
              view.dispatch({
                changes: { from: backwardRange.from, to: backwardRange.to, insert: "" },
                selection: EditorSelection.cursor(backwardRange.from),
              });
              syncMentions(instance);
              return true;
            }
          }
          if (selection.empty && event.key === "Delete") {
            var forwardRange = findMentionRangeAtCursor(
              text,
              instance.mentions,
              selection.head,
              "forward",
            );
            if (forwardRange) {
              event.preventDefault();
              view.dispatch({
                changes: { from: forwardRange.from, to: forwardRange.to, insert: "" },
                selection: EditorSelection.cursor(forwardRange.from),
              });
              syncMentions(instance);
              return true;
            }
          }
          return false;
        },
      });

      var state = EditorState.create({
        doc: "",
        extensions: [
          chatInputTheme,
          EditorView.lineWrapping,
          createMarkdownMentionDecorations(function () {
            return instance.mentions;
          }),
          autocompletion({
            override: [createMentionCompletionSource(instance)],
            activateOnTyping: true,
            activateOnTypingDelay: 0,
            closeOnBlur: true,
            aboveCursor: true,
            icons: false,
            optionClass: function () {
              return "vector-chat-completion-option";
            },
            tooltipClass: function () {
              return "vector-chat-completion";
            },
          }),
          history(),
          keymap.of(defaultKeymap.concat(historyKeymap)),
          cspNonce ? EditorView.cspNonce.of(cspNonce) : [],
          updateListener,
          domHandlers,
        ],
      });

      mountEl.replaceChildren();
      instance.view = new EditorView({ state: state, parent: mountEl });
      instance.view.dom.dataset.chatInputEditable = "true";
      instance.view.dom.setAttribute("role", "textbox");
      if (labelId) {
        instance.view.dom.setAttribute("aria-labelledby", labelId);
      } else {
        instance.view.dom.setAttribute("aria-label", accessibleName);
      }
      instance.view.dom.setAttribute("aria-multiline", "true");
      instance.view.dom.classList.add("vector-chat-input-editor");
      editorsByHost.set(hostEl, instance);
      observeEditorSize(instance);
      scheduleEditorLayout(instance);
    }

    return {
      initialize: function () {
        document.querySelectorAll(".vector-chat-input-host").forEach(function (hostEl) {
          if (!(hostEl instanceof HTMLElement)) return;
          var mountEl = hostEl.querySelector("[data-chat-input-editable]");
          if (!(mountEl instanceof HTMLElement)) return;
          if (editorsByHost.has(hostEl)) return;
          createEditor(hostEl, mountEl);
        });
      },
      handleSuggestionsResult: function (message) {
        if (!message || typeof message.requestId !== "string") {
          return;
        }
        completePendingRequest(message.requestId, message.suggestions || []);
      },
      collectFormValues: function () {
        var values = {};
        document.querySelectorAll("[data-form-key]").forEach(function (field) {
          var key = field.dataset.formKey;
          if (!key) {
            return;
          }
          if (field.classList.contains("vector-form-readonly-value")) {
            values[key] = field.textContent || "";
            return;
          }
          if (field.classList.contains("vector-chat-input-host")) {
            var instance = editorsByHost.get(field);
            values[key] = instance ? instance.view.state.doc.toString() : "";
            return;
          }
          if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
            values[key] = field.value;
          }
        });
        return values;
      },
      collectMentions: function () {
        var result = {};
        editorsByHost.forEach(function (instance) {
          if (!instance.name) return;
          result[instance.name] = reconcileMentions(
            instance.view.state.doc.toString(),
            instance.mentions,
          );
        });
        return result;
      },
      clearEditorByName: function (name) {
        editorsByHost.forEach(function (instance) {
          if (instance.name !== name || !instance.view) return;
          instance.view.dispatch({
            changes: { from: 0, to: instance.view.state.doc.length, insert: "" },
          });
          instance.mentions = [];
        });
      },
    };
  }

  window.VectorChatInputRuntime = {
    create: createChatInputRuntime,
  };
})();
