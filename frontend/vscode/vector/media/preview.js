(function () {
  const vscode = acquireVsCodeApi();
  const tocPanel = document.querySelector("[data-toc-panel]");

  let chatInputRuntime = null;

  function ensureChatInputRuntime() {
    if (chatInputRuntime) {
      return chatInputRuntime;
    }
    if (
      window.VectorChatInputRuntime &&
      typeof window.VectorChatInputRuntime.create === "function"
    ) {
      chatInputRuntime = window.VectorChatInputRuntime.create({ vscode: vscode });
    }
    return chatInputRuntime;
  }

  function initializeChatInputRuntime() {
    var runtime = ensureChatInputRuntime();
    if (runtime && typeof runtime.initialize === "function") {
      runtime.initialize();
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", initializeChatInputRuntime, { once: true });
  } else {
    initializeChatInputRuntime();
  }

  // --- inline action overlay ---

  var overlayEl = null;
  var overlayPendingAction = null;
  var overlayFormRequestId = null;
  var overlayFormRequestCounter = 0;

  var RENDER_FORM_BLOCK_REQUEST_TYPE = "vector.renderFormBlock";
  var RENDER_FORM_BLOCK_RESULT_TYPE = "vector.renderFormBlockResult";
  var OVERLAY_FORM_CONTENT = 'prompt-message = chat-input("Prompt")';

  function buildInlineActionOverlay() {
    var backdrop = document.createElement("div");
    backdrop.className = "vector-inline-overlay-backdrop";

    var infoControl = document.createElement("div");
    infoControl.className = "vector-inline-overlay-info";
    infoControl.setAttribute("role", "button");
    infoControl.setAttribute("tabindex", "0");
    infoControl.setAttribute("aria-label", "Action information — click to run");

    var infoLabel = document.createElement("span");
    infoLabel.className = "vector-inline-overlay-info-label";
    infoControl.appendChild(infoLabel);

    var messageField = document.createElement("div");
    messageField.className = "vector-inline-overlay-message-field";

    var cancelBtn = document.createElement("button");
    cancelBtn.type = "button";
    cancelBtn.className = "vector-inline-overlay-cancel";
    cancelBtn.textContent = "Cancel";

    var submitBtn = document.createElement("button");
    submitBtn.type = "button";
    submitBtn.className = "vector-inline-overlay-submit";
    submitBtn.textContent = "Run";

    var footer = document.createElement("div");
    footer.className = "vector-inline-overlay-footer";
    footer.appendChild(cancelBtn);
    footer.appendChild(submitBtn);

    var panel = document.createElement("div");
    panel.className = "vector-inline-overlay-panel";
    panel.setAttribute("role", "dialog");
    panel.setAttribute("aria-modal", "true");
    panel.appendChild(infoControl);
    panel.appendChild(messageField);
    panel.appendChild(footer);

    var container = document.createElement("div");
    container.className = "vector-inline-overlay";
    container.appendChild(backdrop);
    container.appendChild(panel);

    var focusables = [infoControl, cancelBtn, submitBtn];

    function submitOverlay() {
      if (!overlayPendingAction) return;
      var action = overlayPendingAction;
      var runtime = ensureChatInputRuntime();
      var extra = "";
      var chatInputMentions = {};
      if (runtime) {
        if (typeof runtime.collectFormValues === "function") {
          extra = (runtime.collectFormValues()["prompt-message"] || "").trim();
        }
        if (typeof runtime.collectMentions === "function") {
          chatInputMentions = runtime.collectMentions();
        }
      }
      var staticInput = Object.assign({}, action.staticInput);
      if (extra) {
        staticInput["prompt-message"] = extra;
      }
      closeInlineOverlay();
      vscode.postMessage({
        type: "vector.runAgent",
        profile: action.profile,
        prompt: action.prompt,
        label: action.label,
        staticInput: staticInput,
        formValues: action.formValues,
        chatInputMentions: chatInputMentions,
      });
    }

    container.addEventListener("keydown", function (e) {
      if (e.key === "Escape") {
        e.preventDefault();
        closeInlineOverlay();
        return;
      }
      if (e.key === "Tab") {
        var active = document.activeElement;
        var idx = focusables.indexOf(active);
        if (idx === -1) return;
        e.preventDefault();
        if (e.shiftKey) {
          focusables[(idx - 1 + focusables.length) % focusables.length].focus();
        } else {
          focusables[(idx + 1) % focusables.length].focus();
        }
      }
    });

    backdrop.addEventListener("click", function () {
      closeInlineOverlay();
    });

    cancelBtn.addEventListener("click", function () {
      closeInlineOverlay();
    });

    submitBtn.addEventListener("click", submitOverlay);

    infoControl.addEventListener("click", submitOverlay);
    infoControl.addEventListener("keydown", function (e) {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        submitOverlay();
      }
    });

    document.body.appendChild(container);

    return {
      container: container,
      infoLabel: infoLabel,
      infoControl: infoControl,
      messageField: messageField,
      focusables: focusables,
      permanentFocusables: [infoControl, cancelBtn, submitBtn],
    };
  }

  function getOrCreateOverlay() {
    if (!overlayEl) {
      overlayEl = buildInlineActionOverlay();
    }
    return overlayEl;
  }

  function openInlineOverlay(action) {
    overlayPendingAction = action;
    var o = getOrCreateOverlay();
    o.infoLabel.textContent = action.label + " — " + action.prompt;
    o.infoControl.setAttribute("aria-label", "Run: " + action.label);
    o.container.classList.add("is-open");

    var requestId = "overlay-form-" + (++overlayFormRequestCounter);
    overlayFormRequestId = requestId;
    vscode.postMessage({
      type: RENDER_FORM_BLOCK_REQUEST_TYPE,
      requestId: requestId,
      content: OVERLAY_FORM_CONTENT,
    });
  }

  function handleRenderFormBlockResult(msg) {
    if (!overlayEl || msg.requestId !== overlayFormRequestId) return;
    overlayFormRequestId = null;
    var o = overlayEl;
    o.messageField.innerHTML = msg.html;
    var runtime = ensureChatInputRuntime();
    if (runtime && typeof runtime.initialize === "function") {
      runtime.initialize();
    }
    var chatHost = o.messageField.querySelector(".vector-chat-input-host");
    var cmContent = chatHost ? chatHost.querySelector(".cm-content") : null;
    if (cmContent instanceof HTMLElement) {
      var insertIdx = o.focusables.indexOf(o.infoControl) + 1;
      if (!o.focusables.includes(cmContent)) {
        o.focusables.splice(insertIdx, 0, cmContent);
      }
      cmContent.focus();
    } else if (chatHost instanceof HTMLElement) {
      chatHost.focus();
    }
  }

  function closeInlineOverlay() {
    overlayPendingAction = null;
    overlayFormRequestId = null;
    if (overlayEl) {
      overlayEl.container.classList.remove("is-open");
      overlayEl.messageField.innerHTML = "";
      overlayEl.focusables.length = 0;
      overlayEl.permanentFocusables.forEach(function (el) {
        overlayEl.focusables.push(el);
      });
      var runtime = ensureChatInputRuntime();
      if (runtime && typeof runtime.clearEditorByName === "function") {
        runtime.clearEditorByName("prompt-message");
      }
    }
  }

  // --- toc ---

  function closeToc() {
    if (tocPanel instanceof HTMLElement) {
      tocPanel.setAttribute("hidden", "true");
    }
  }

  function openToc() {
    if (tocPanel instanceof HTMLElement) {
      tocPanel.removeAttribute("hidden");
    }
  }

  function toggleToc() {
    if (tocPanel instanceof HTMLElement) {
      if (tocPanel.hasAttribute("hidden")) {
        openToc();
      } else {
        closeToc();
      }
    }
  }

  document.addEventListener("click", function (event) {
    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }

    const agentTrigger = target.closest("[data-agent-profile]");
    if (agentTrigger instanceof HTMLElement) {
      event.preventDefault();
      const profile = agentTrigger.dataset.agentProfile;
      const prompt = agentTrigger.dataset.agentPrompt;
      const label = agentTrigger.dataset.agentLabel ?? "";
      const inputJson = agentTrigger.dataset.agentInput;
      let staticInput = {};
      try {
        staticInput = inputJson ? JSON.parse(inputJson) : {};
      } catch {
        staticInput = {};
      }

      if (agentTrigger.classList.contains("vector-agent-inline-action")) {
        const formValues = typeof window.vectorCollectFormValues === "function"
          ? window.vectorCollectFormValues()
          : {};
        if (profile && prompt) {
          openInlineOverlay({ profile: profile, prompt: prompt, label: label, staticInput: staticInput, formValues: formValues });
        }
        return;
      }

      const formValues = typeof window.vectorCollectFormValues === "function"
        ? window.vectorCollectFormValues()
        : {};
      const chatInputMentions = typeof window.vectorCollectChatInputMentions === "function"
        ? window.vectorCollectChatInputMentions()
        : {};
      if (profile && prompt) {
        vscode.postMessage({
          type: "vector.runAgent",
          profile: profile,
          prompt: prompt,
          label: label,
          staticInput: staticInput,
          formValues: formValues,
          chatInputMentions: chatInputMentions,
        });
      }
      return;
    }

    const openDoc = target.closest("a[data-open-doc]");
    if (openDoc instanceof HTMLAnchorElement) {
      event.preventDefault();
      const doc = openDoc.dataset.openDoc;
      const inputJson = openDoc.dataset.openDocInput;
      let input = {};
      try {
        input = inputJson ? JSON.parse(inputJson) : {};
      } catch {
        input = {};
      }
      if (doc) {
        vscode.postMessage({ type: "vector.openDoc", doc: doc, input: input });
      }
      return;
    }

    const wikilink = target.closest("a[data-wikilink]");
    if (wikilink instanceof HTMLAnchorElement) {
      event.preventDefault();
      const stem = wikilink.dataset.wikilink;
      if (stem) {
        vscode.postMessage({ type: "vector.navigateWikilink", stem: stem });
      }
      return;
    }

    const fmlink = target.closest("a[data-fmlink]");
    if (fmlink instanceof HTMLAnchorElement) {
      event.preventDefault();
      const stem = fmlink.dataset.fmlink;
      if (stem) {
        vscode.postMessage({ type: "vector.navigateFmLink", stem: stem });
      }
      return;
    }

    const tocItem = target.closest("button[data-heading-id]");
    if (tocItem instanceof HTMLButtonElement) {
      event.preventDefault();
      const headingId = tocItem.dataset.headingId;
      if (headingId) {
        const heading = document.getElementById(headingId);
        if (heading instanceof HTMLElement) {
          heading.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      }
      closeToc();
      return;
    }

    if (tocPanel instanceof HTMLElement && !tocPanel.hasAttribute("hidden")) {
      if (!tocPanel.contains(target)) {
        closeToc();
      }
    }
  });

  document.addEventListener("change", function (event) {
    const target = event.target;
    if (!(target instanceof HTMLSelectElement)) {
      return;
    }

    if (target.matches("[data-status-select]")) {
      vscode.postMessage({ type: "vector.changeStatus", status: target.value });
    }
  });

  window.addEventListener("message", function (event) {
    const msg = event.data;
    if (msg && msg.type === "vector.toggleToc") {
      toggleToc();
      return;
    }
    if (
      msg &&
      msg.type === "vector.chatInput.suggestionsResult" &&
      ensureChatInputRuntime() &&
      typeof chatInputRuntime.handleSuggestionsResult === "function"
    ) {
      chatInputRuntime.handleSuggestionsResult(msg);
      return;
    }
    if (msg && msg.type === RENDER_FORM_BLOCK_RESULT_TYPE) {
      handleRenderFormBlockResult(msg);
    }
  });

  document.querySelectorAll('pre code[class*="language-"]').forEach(function (block) {
    if (typeof hljs !== "undefined") {
      hljs.highlightElement(block);
    }
  });

  function collectFormValues() {
    var runtime = ensureChatInputRuntime();
    if (runtime && typeof runtime.collectFormValues === "function") {
      return chatInputRuntime.collectFormValues();
    }

    var values = {};
    document.querySelectorAll("[data-form-key]").forEach(function (field) {
      var key = field.dataset.formKey;
      if (!key) {
        return;
      }
      if (field.classList.contains("vector-form-readonly-value")) {
        values[key] = field.textContent ?? "";
        return;
      }
      if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
        values[key] = field.value;
      }
    });
    return values;
  }

  function collectChatInputMentions() {
    var runtime = ensureChatInputRuntime();
    if (runtime && typeof runtime.collectMentions === "function") {
      return chatInputRuntime.collectMentions();
    }
    return {};
  }

  window.vectorCollectFormValues = collectFormValues;
  window.vectorCollectChatInputMentions = collectChatInputMentions;
})();
