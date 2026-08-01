// The bar pinned to the bottom of the page is ommp's own status bar. It reads
// the page the way the player reads a track: how far in you are, and what is
// playing right now. Nothing here is required to read the page — if this file
// never loads, the bar simply sits at 0:00.
(function () {
  "use strict";

  var fill = document.querySelector(".sb-fill");
  var track = document.querySelector(".sb-track");
  var elapsed = document.querySelector(".sb-elapsed");
  if (!fill || !track || !elapsed) return;

  var TOTAL = 237; // 3:57, the length of the track in the screenshot
  var sections = Array.prototype.slice.call(document.querySelectorAll("[data-section]"));

  function clock(seconds) {
    var s = Math.round(seconds);
    return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0");
  }

  function update() {
    var scrollable = document.documentElement.scrollHeight - window.innerHeight;
    var progress = scrollable > 0 ? Math.min(1, Math.max(0, window.scrollY / scrollable)) : 0;

    fill.style.width = (progress * 100).toFixed(2) + "%";
    elapsed.textContent = clock(progress * TOTAL);

    // Whichever labelled section has most recently crossed the middle of the
    // viewport is the one you are reading.
    var middle = window.innerHeight / 2;
    var current = "ommp";
    for (var i = 0; i < sections.length; i++) {
      if (sections[i].getBoundingClientRect().top <= middle) {
        current = sections[i].getAttribute("data-section");
      }
    }
    if (track.textContent !== current) track.textContent = current;
  }

  var queued = false;
  function onScroll() {
    if (queued) return;
    queued = true;
    requestAnimationFrame(function () { queued = false; update(); });
  }

  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("resize", onScroll);
  update();

  // Copy a command. The button is its own element beside the box, so the box
  // never resizes — the icon swaps and that is the whole feedback.
  document.querySelectorAll(".cmd-copy").forEach(function (btn) {
    var cmd = btn.parentNode.querySelector(".cmd");
    if (!cmd) return;
    btn.addEventListener("click", function () {
      if (!navigator.clipboard) return;
      // textContent, so the "$ " prompt drawn by CSS is left behind.
      navigator.clipboard.writeText(cmd.textContent.trim()).then(function () {
        btn.classList.add("is-copied");
        setTimeout(function () { btn.classList.remove("is-copied"); }, 1600);
      });
    });
  });

  // The setup you skip crosses itself out as you reach it. The markup ships
  // already struck, so this first un-strikes it and then puts the lines back
  // one at a time; if the script never runs, the finished state is what shows.
  var setup = document.querySelector(".setup-skipped");
  if (setup && "IntersectionObserver" in window) {
    var lines = Array.prototype.slice.call(setup.children);
    setup.parentNode.classList.add("setup-armed");

    var still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    var strike = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (!e.isIntersecting) return;
        strike.unobserve(e.target);
        lines.forEach(function (li, i) {
          setTimeout(function () { li.classList.add("is-struck"); }, still ? 0 : i * 260);
        });
      });
    }, { threshold: 0.6 });

    strike.observe(setup);
  }
})();
