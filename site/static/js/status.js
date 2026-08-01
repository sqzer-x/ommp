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

  // Click a command to copy it — the page is mostly commands.
  document.querySelectorAll("[data-copy]").forEach(function (el) {
    el.addEventListener("click", function () {
      if (!navigator.clipboard) return;
      navigator.clipboard.writeText(el.textContent.trim()).then(function () {
        el.classList.add("is-copied");
        setTimeout(function () { el.classList.remove("is-copied"); }, 1400);
      });
    });
  });
})();
