/* ============================================
   Zamin Documentation - Interactive JavaScript
   ============================================ */

(function() {
  'use strict';

  // === Theme Toggle ===
  const THEME_KEY = 'zamin-docs-theme';
  const html = document.documentElement;

  function getPreferredTheme() {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored) return stored;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function setTheme(theme) {
    html.setAttribute('data-theme', theme);
    localStorage.setItem(THEME_KEY, theme);
    const btn = document.getElementById('theme-toggle');
    if (btn) btn.textContent = theme === 'dark' ? '\u2600' : '\u263E';
  }

  setTheme(getPreferredTheme());

  document.addEventListener('DOMContentLoaded', function() {
    const themeBtn = document.getElementById('theme-toggle');
    if (themeBtn) {
      themeBtn.addEventListener('click', function() {
        const current = html.getAttribute('data-theme') || 'light';
        setTheme(current === 'dark' ? 'light' : 'dark');
      });
    }

    // === Mobile Menu ===
    const menuBtn = document.getElementById('mobile-menu-btn');
    const sidebar = document.getElementById('sidebar');
    const overlay = document.getElementById('sidebar-overlay');

    if (menuBtn && sidebar) {
      menuBtn.addEventListener('click', function() {
        sidebar.classList.toggle('open');
        if (overlay) overlay.classList.toggle('active');
      });
      if (overlay) {
        overlay.addEventListener('click', function() {
          sidebar.classList.remove('open');
          overlay.classList.remove('active');
        });
      }
    }

    // === Copy Code ===
    document.querySelectorAll('.copy-btn').forEach(function(btn) {
      btn.addEventListener('click', function() {
        const codeBlock = btn.closest('.code-header')
          ? btn.closest('.code-header').nextElementSibling
          : btn.closest('pre');
        if (!codeBlock) return;
        const code = codeBlock.querySelector('code') || codeBlock;
        navigator.clipboard.writeText(code.textContent).then(function() {
          btn.textContent = 'Copied!';
          btn.classList.add('copied');
          setTimeout(function() {
            btn.textContent = 'Copy';
            btn.classList.remove('copied');
          }, 2000);
        });
      });
    });

    // === Search ===
    const searchInput = document.getElementById('search-input');
    const searchResults = document.getElementById('search-results');
    let searchIndex = null;

    if (searchInput && searchResults) {
      fetch('assets/search-index.json')
        .then(function(r) { return r.json(); })
        .then(function(data) { searchIndex = data; })
        .catch(function() { /* Search unavailable */ });

      searchInput.addEventListener('input', function() {
        const query = searchInput.value.trim().toLowerCase();
        if (query.length < 2 || !searchIndex) {
          searchResults.classList.remove('active');
          return;
        }

        const results = searchIndex.filter(function(item) {
          return item.title.toLowerCase().includes(query) ||
                 item.content.toLowerCase().includes(query) ||
                 (item.section && item.section.toLowerCase().includes(query));
        }).slice(0, 8);

        if (results.length === 0) {
          searchResults.innerHTML = '<div class="search-result-item"><div class="result-title">No results found</div></div>';
        } else {
          searchResults.innerHTML = results.map(function(r) {
            return '<a href="' + r.url + '" class="search-result-item">' +
              '<div class="result-title">' + escapeHtml(r.title) + '</div>' +
              '<div class="result-section">' + escapeHtml(r.section || '') + '</div>' +
            '</a>';
          }).join('');
        }
        searchResults.classList.add('active');
      });

      searchInput.addEventListener('blur', function() {
        setTimeout(function() { searchResults.classList.remove('active'); }, 200);
      });
    }

    // === Smooth Scroll for Anchor Links ===
    document.querySelectorAll('a[href^="#"]').forEach(function(link) {
      link.addEventListener('click', function(e) {
        const id = link.getAttribute('href').slice(1);
        const target = document.getElementById(id);
        if (target) {
          e.preventDefault();
          target.scrollIntoView({ behavior: 'smooth', block: 'start' });
          history.pushState(null, null, '#' + id);
        }
      });
    });

    // === Active TOC Link ===
    const tocLinks = document.querySelectorAll('.toc-link');
    if (tocLinks.length > 0) {
      const headings = [];
      tocLinks.forEach(function(link) {
        const id = link.getAttribute('href').replace('#', '');
        const el = document.getElementById(id);
        if (el) headings.push({ el: el, link: link });
      });

      function updateToc() {
        let current = headings[0];
        for (let i = 0; i < headings.length; i++) {
          if (headings[i].el.getBoundingClientRect().top <= 100) {
            current = headings[i];
          }
        }
        tocLinks.forEach(function(l) { l.classList.remove('active'); });
        if (current) current.link.classList.add('active');
      }

      window.addEventListener('scroll', updateToc);
      updateToc();
    }

    // === Feedback Widget ===
    document.querySelectorAll('.feedback button').forEach(function(btn) {
      btn.addEventListener('click', function() {
        document.querySelectorAll('.feedback button').forEach(function(b) {
          b.classList.remove('active');
        });
        btn.classList.add('active');
        const msg = document.getElementById('feedback-msg');
        if (msg) msg.textContent = 'Thanks for your feedback!';
      });
    });

    // === Module Card Toggle ===
    document.querySelectorAll('.module-card-header').forEach(function(header) {
      header.addEventListener('click', function() {
        const body = header.nextElementSibling;
        const icon = header.querySelector('.toggle-icon');
        if (body) {
          body.classList.toggle('collapsed');
          if (icon) icon.textContent = body.classList.contains('collapsed') ? '\u25B6' : '\u25BC';
        }
      });
    });

    // === Active Sidebar Link ===
    const currentPath = window.location.pathname.split('/').pop() || 'index.html';
    document.querySelectorAll('.sidebar-link').forEach(function(link) {
      const href = link.getAttribute('href');
      if (href && href.split('/').pop() === currentPath) {
        link.classList.add('active');
      }
    });

    // === Print Button ===
    const printBtn = document.getElementById('print-btn');
    if (printBtn) {
      printBtn.addEventListener('click', function() { window.print(); });
    }

    // === GitHub Link ===
    const editBtn = document.getElementById('edit-link');
    if (editBtn) {
      const page = window.location.pathname.split('/').pop() || 'index.html';
      editBtn.href = 'https://github.com/young-developer90/zamin/edit/master/docs/' + page;
    }
  });

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
})();
