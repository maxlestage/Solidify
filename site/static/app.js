// Solidify's site: motion and small conveniences, in plain JavaScript, no dependency.
//
// The page works without this file: every link, the language picker (`<details>`) and the
// playground (WebAssembly, see islands.rs) do their job alone. This file adds:
//
//   1. the language: remembers the one picked, "Automatic" follows the browser again;
//   2. the menu on small screens;
//   3. the header that tightens once the page scrolls, the reading progress bar, and the
//      menu link of the section being read;
//   4. reveals: elements marked `data-reveal` appear when they scroll into view
//      (`words`: a heading's words rise in turn, `rows`: a table's rows, `scale`: grows);
//   5. counters: the hero's numbers count up;
//   6. the spotlight that follows the pointer on cards (`.spot`) and the light under it;
//   7. magnetic buttons (`.magnetic`) drawn a little towards the pointer;
//   8. the hero's parallax and the wordmark of the footer;
//   9. copy buttons (`data-copy`), and a soft fade when the language changes.
//
// When the system asks for less motion (`prefers-reduced-motion`), nothing moves:
// everything shows at once, and the CSS turns animations off.
(function () {
  "use strict";

  var root = document.documentElement;
  var still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var finePointer = window.matchMedia("(pointer: fine)").matches;
  var STORE = "solidify-lang";

  // Scroll reveals hide what is below until it comes into view: only once this file runs,
  // so that a page without it shows everything.
  if (!still) root.classList.add("motion");

  // Coming back with the browser's Back button: the page is shown again, not faded.
  window.addEventListener("pageshow", function () {
    root.classList.remove("leaving");
  });

  function store(value) {
    try {
      if (value === null) localStorage.removeItem(STORE);
      else localStorage.setItem(STORE, value);
    } catch (e) {
      /* private browsing: the choice lasts for this page only */
    }
  }

  /* 1. Language ---------------------------------------------------------------------- */

  // Every language link keeps the choice; "auto" forgets it and lets the browser decide
  // (the script in <head> then picks the page).
  document.addEventListener("click", function (event) {
    var a = event.target.closest && event.target.closest("a[data-lang]");
    if (!a) return;
    var lang = a.getAttribute("data-lang");
    store(lang === "auto" ? null : lang);
    if (still || event.metaKey || event.ctrlKey || event.shiftKey) return;
    event.preventDefault();
    root.classList.add("leaving");
    setTimeout(function () {
      location.href = a.href + location.hash;
    }, 260);
  });

  // The picker closes on a click elsewhere or on Escape.
  var picker = document.querySelector(".lang-picker");
  document.addEventListener("click", function (event) {
    if (picker && picker.open && !picker.contains(event.target)) picker.open = false;
  });

  /* 2. Menu on small screens --------------------------------------------------------- */

  var toggle = document.querySelector(".menu-toggle");
  var menu = document.getElementById("menu");
  function setMenu(open) {
    if (!toggle) return;
    root.classList.toggle("menu-open", open);
    toggle.setAttribute("aria-expanded", String(open));
    toggle.setAttribute(
      "aria-label",
      toggle.getAttribute(open ? "data-close" : "data-open")
    );
  }
  if (toggle && menu) {
    toggle.addEventListener("click", function () {
      setMenu(!root.classList.contains("menu-open"));
    });
    menu.addEventListener("click", function (event) {
      if (event.target.closest("a")) setMenu(false);
    });
  }
  document.addEventListener("keydown", function (event) {
    if (event.key !== "Escape") return;
    setMenu(false);
    if (picker) picker.open = false;
  });

  /* 3. Header, progress, current section --------------------------------------------- */

  var header = document.querySelector(".top");
  var progress = document.querySelector(".progress");
  var hero = document.querySelector(".hero");
  var wordmark = document.querySelector(".wordmark");
  var ticking = false;

  function onScroll() {
    ticking = false;
    var y = window.scrollY;
    var max = document.documentElement.scrollHeight - window.innerHeight;
    if (header) header.classList.toggle("scrolled", y > 8);
    if (progress) progress.style.transform = "scaleX(" + (max > 0 ? y / max : 0) + ")";
    if (still) return;
    // 8. Parallax: the hero's light drifts slower than the page.
    if (hero && y < window.innerHeight * 1.2) {
      hero.style.setProperty("--py", (y * 0.25).toFixed(1) + "px");
    }
    // The footer's wordmark slides in as the end of the page comes.
    if (wordmark) {
      var r = wordmark.getBoundingClientRect();
      var seen = 1 - Math.min(Math.max(r.top / window.innerHeight, 0), 1);
      wordmark.style.setProperty("--seen", seen.toFixed(3));
    }
  }
  window.addEventListener(
    "scroll",
    function () {
      if (!ticking) {
        ticking = true;
        requestAnimationFrame(onScroll);
      }
    },
    { passive: true }
  );
  onScroll();

  // The menu link of the section in view.
  var spyLinks = document.querySelectorAll("[data-spy]");
  if ("IntersectionObserver" in window && spyLinks.length) {
    var spy = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (!entry.isIntersecting) return;
          spyLinks.forEach(function (l) {
            l.classList.toggle("on", l.getAttribute("data-spy") === entry.target.id);
          });
        });
      },
      { rootMargin: "-45% 0px -50% 0px" }
    );
    spyLinks.forEach(function (l) {
      var section = document.getElementById(l.getAttribute("data-spy"));
      if (section) spy.observe(section);
    });
  }

  /* 4. Reveals ------------------------------------------------------------------------ */

  var reveals = document.querySelectorAll("[data-reveal]");
  // Table rows get their order, for a staggered entrance (capped so a long table is quick).
  document.querySelectorAll('[data-reveal="rows"] tbody tr').forEach(function (tr, i) {
    tr.style.setProperty("--i", String(Math.min(i, 16)));
  });
  if (still || !("IntersectionObserver" in window)) {
    reveals.forEach(function (el) {
      el.classList.add("in");
    });
  } else {
    var seen = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (!entry.isIntersecting) return;
          entry.target.classList.add("in");
          seen.unobserve(entry.target);
        });
      },
      { rootMargin: "0px 0px -8% 0px", threshold: 0.08 }
    );
    reveals.forEach(function (el) {
      seen.observe(el);
    });
  }

  /* 5. Counters ------------------------------------------------------------------------ */

  document.querySelectorAll("[data-count]").forEach(function (el) {
    var target = parseInt(el.getAttribute("data-count"), 10);
    if (still || !(target > 1)) return;
    var start = null;
    var duration = 1400;
    el.textContent = "0";
    function step(now) {
      if (start === null) start = now;
      var t = Math.min((now - start) / duration, 1);
      var eased = 1 - Math.pow(1 - t, 4);
      el.textContent = String(Math.round(target * eased));
      if (t < 1) requestAnimationFrame(step);
    }
    // After the hero's entrance.
    setTimeout(function () {
      requestAnimationFrame(step);
    }, 900);
  });

  /* 6. Spotlight and the light under the pointer --------------------------------------- */

  if (finePointer && !still) {
    var glow = document.querySelector(".glow");
    window.addEventListener(
      "pointermove",
      function (event) {
        if (glow) {
          glow.style.transform =
            "translate(" + (event.clientX - 300) + "px," + (event.clientY - 300) + "px)";
          glow.classList.add("on");
        }
        var card = event.target.closest && event.target.closest(".spot");
        if (card) {
          var r = card.getBoundingClientRect();
          card.style.setProperty("--mx", event.clientX - r.left + "px");
          card.style.setProperty("--my", event.clientY - r.top + "px");
        }
      },
      { passive: true }
    );

    /* 7. Magnetic buttons ---------------------------------------------------------------- */

    document.querySelectorAll(".magnetic").forEach(function (button) {
      button.addEventListener("pointermove", function (event) {
        var r = button.getBoundingClientRect();
        var x = (event.clientX - r.left - r.width / 2) * 0.25;
        var y = (event.clientY - r.top - r.height / 2) * 0.35;
        button.style.transform = "translate(" + x + "px," + y + "px)";
      });
      button.addEventListener("pointerleave", function () {
        button.style.transform = "";
      });
    });
  }

  /* 9. Copy buttons -------------------------------------------------------------------- */

  document.addEventListener("click", function (event) {
    var button = event.target.closest && event.target.closest("[data-copy]");
    if (!button || !navigator.clipboard) return;
    navigator.clipboard.writeText(button.getAttribute("data-copy")).then(function () {
      var label = button.textContent;
      button.textContent = button.getAttribute("data-done") || label;
      button.classList.add("done");
      setTimeout(function () {
        button.textContent = label;
        button.classList.remove("done");
      }, 1600);
    });
  });

  // Everything is in place: the entrance animations of the hero may start.
  requestAnimationFrame(function () {
    root.classList.add("ready");
  });
})();
