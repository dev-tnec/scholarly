// Scholarly progressive enhancement: carousel, filter sheet, FAQ accordion,
// nav toggle. All optional: without JS the pages render and every control
// still works through native elements or plain links (PRD section 7).
(function () {
  'use strict';
  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  document.documentElement.classList.add('js');

  /* ---------------------------------------------------------------- nav */
  var navToggle = document.querySelector('[data-nav-toggle]');
  var nav = document.getElementById('siteNav');
  if (navToggle && nav) {
    navToggle.addEventListener('click', function () {
      var open = nav.classList.toggle('is-open');
      navToggle.setAttribute('aria-expanded', open ? 'true' : 'false');
    });
  }

  /* ---------------------------------------------------------------- carousel */
  var carousel = document.querySelector('[data-carousel]');
  if (carousel) {
    var slides = Array.prototype.slice.call(carousel.querySelectorAll('.hero-slide'));
    var dots = Array.prototype.slice.call(
      document.querySelectorAll('[data-carousel-dots] .hero-dot')
    );
    var counter = document.querySelector('[data-carousel-counter]');
    var current = 0;
    var timer = null;

    function pad(n) { return n < 10 ? '0' + n : '' + n; }

    function show(index) {
      current = (index + slides.length) % slides.length;
      slides.forEach(function (slide, i) {
        slide.classList.toggle('is-active', i === current);
      });
      dots.forEach(function (dot, i) {
        dot.classList.toggle('is-active', i === current);
      });
      if (counter) counter.textContent = pad(current + 1) + ' / ' + pad(slides.length);
    }

    function start() {
      if (reduced || slides.length < 2) return;
      stop();
      timer = window.setInterval(function () { show(current + 1); }, 6000);
    }

    function stop() {
      if (timer) { window.clearInterval(timer); timer = null; }
    }

    var prev = document.querySelector('[data-carousel-prev]');
    var next = document.querySelector('[data-carousel-next]');
    if (prev) prev.addEventListener('click', function () { show(current - 1); start(); });
    if (next) next.addEventListener('click', function () { show(current + 1); start(); });
    dots.forEach(function (dot, i) {
      dot.addEventListener('click', function () { show(i); start(); });
    });

    carousel.addEventListener('mouseenter', stop);
    carousel.addEventListener('mouseleave', start);
    carousel.addEventListener('focusin', stop);
    carousel.addEventListener('focusout', start);

    // Swipe on touch devices.
    var startX = null;
    carousel.addEventListener('touchstart', function (e) {
      startX = e.touches[0].clientX;
    }, { passive: true });
    carousel.addEventListener('touchend', function (e) {
      if (startX === null) return;
      var dx = e.changedTouches[0].clientX - startX;
      if (Math.abs(dx) > 40) show(dx < 0 ? current + 1 : current - 1);
      startX = null;
      start();
    }, { passive: true });

    // Keyboard arrows while the carousel holds focus.
    carousel.addEventListener('keydown', function (e) {
      if (e.key === 'ArrowLeft') { show(current - 1); start(); }
      if (e.key === 'ArrowRight') { show(current + 1); start(); }
    });

    show(0);
    start();
  }

  /* ---------------------------------------------------------------- filter sheet */
  var rail = document.getElementById('filterRail');
  var scrim = document.getElementById('filterScrim');
  var filterBtns = document.querySelectorAll('[data-filter-open]');
  var closeBtn = document.getElementById('filterClose');

  function setSheet(open) {
    if (!rail) return;
    if (window.innerWidth >= 1024) open = false;
    rail.classList.toggle('is-open', open);
    if (scrim) scrim.classList.toggle('is-open', open);
    document.body.classList.toggle('sheet-lock', open);
    filterBtns.forEach(function (btn) {
      btn.setAttribute('aria-expanded', open ? 'true' : 'false');
    });
  }

  Array.prototype.forEach.call(filterBtns, function (btn) {
    btn.addEventListener('click', function () {
      setSheet(!rail.classList.contains('is-open'));
    });
  });
  if (closeBtn) closeBtn.addEventListener('click', function () { setSheet(false); });
  if (scrim) scrim.addEventListener('click', function () { setSheet(false); });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') setSheet(false);
  });
  window.addEventListener('resize', function () {
    if (window.innerWidth >= 1024) setSheet(false);
  });

  // Auto-submit the filter form when a checkbox changes; the submit button
  // remains the no-JS path.
  var filterForm = document.querySelector('[data-filter-form]');
  if (filterForm) {
    filterForm.addEventListener('change', function () { filterForm.submit(); });
  }

  // Country search inside the negara group.
  var search = document.querySelector('[data-filter-search]');
  if (search) {
    search.addEventListener('input', function () {
      var needle = search.value.toLowerCase();
      var list = search.closest('fieldset').querySelector('[data-filter-options]');
      Array.prototype.forEach.call(list.querySelectorAll('.filter-option'), function (option) {
        var label = option.querySelector('.filter-option-label');
        var text = label ? label.textContent.toLowerCase() : '';
        option.style.display = text.indexOf(needle) !== -1 ? '' : 'none';
      });
    });
  }

  // Sort select submits on change; the noscript button covers JS off.
  Array.prototype.forEach.call(document.querySelectorAll('[data-autosubmit]'), function (el) {
    el.addEventListener('change', function () { el.form.submit(); });
  });

  /* ---------------------------------------------------------------- FAQ accordion */
  var faq = document.querySelector('.faq');
  if (faq) {
    faq.addEventListener('toggle', function (e) {
      var details = e.target;
      if (details.tagName !== 'DETAILS' || !details.open) return;
      Array.prototype.forEach.call(faq.querySelectorAll('details'), function (other) {
        if (other !== details) other.open = false;
      });
    }, true);
  }

  /* ---------------------------------------------------------------- reveal */
  if (!reduced && 'IntersectionObserver' in window) {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (!entry.isIntersecting) return;
        entry.target.animate(
          [{ opacity: 0, transform: 'translateY(24px)' }, { opacity: 1, transform: 'none' }],
          { duration: 420, easing: 'cubic-bezier(.16,1,.3,1)', fill: 'both' }
        );
        io.unobserve(entry.target);
      });
    }, { threshold: 0.12 });
    document.querySelectorAll('.reveal').forEach(function (el) { io.observe(el); });
  }
})();
