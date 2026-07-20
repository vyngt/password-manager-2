// e2e console-clean seam (slice 2.9.2, broadened 5.3.1d). Buffers console.warn /
// console.error PLUS uncaught exceptions and rejected promises into a bounded ring
// so the WebDriver harness (crates/vedge-e2e) can assert the app produced no Leptos
// "outside a reactive tracking context" warning AND no runtime fault (a wasm panic
// surfaces as `Uncaught RuntimeError: unreachable`; a glue fault as an
// `Uncaught TypeError` — neither goes through console.*).
//
// Moved out of index.html into this external file so a strict `script-src 'self'`
// CSP (PG.2b) covers it without an inline-script exception. Loaded as a classic
// (non-module) script before the Trunk `rel="rust"` wasm module, which Trunk emits
// deferred — so a parser-blocking classic script still runs first and captures
// boot-time faults. Inert in production: it only appends to an array and forwards
// to the original console method.
(function () {
    var CAP = 200;
    var buf = (window.__vedge_warnings = []);
    function record(level, text) {
        try {
            buf.push({ level: level, text: String(text) });
            if (buf.length > CAP) buf.shift();
        } catch (_) {}
    }
    ["warn", "error"].forEach(function (level) {
        var orig = console[level] ? console[level].bind(console) : function () {};
        console[level] = function () {
            try {
                record(
                    level,
                    Array.prototype.map
                        .call(arguments, function (a) {
                            return String(a);
                        })
                        .join(" "),
                );
            } catch (_) {}
            return orig.apply(console, arguments);
        };
    });
    // Bubble-phase `error` sees uncaught script exceptions (incl. wasm traps
    // re-thrown by wasm-bindgen) but NOT resource-load errors (those don't bubble
    // to window), so this won't false-positive on a missing asset. Rejections cover
    // a `.catch`-less async fault.
    window.addEventListener("error", function (e) {
        if (e && (e.message || e.error)) {
            record("uncaught", e.message || e.error);
        }
    });
    window.addEventListener("unhandledrejection", function (e) {
        var r = e && e.reason;
        record("rejection", (r && (r.stack || r.message)) || r || "unhandledrejection");
    });
})();
