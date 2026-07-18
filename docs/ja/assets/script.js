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

    // === Sidebar Resize & Collapse ===
    var sidebarGrip = document.getElementById('sidebar-grip');
    var sidebarToggle = document.getElementById('sidebar-toggle');
    var root = document.documentElement;

    function getSidebarWidth() {
      return parseInt(localStorage.getItem('zamin-sidebar-width')) || 280;
    }

    function setSidebarWidth(px) {
      px = Math.max(160, Math.min(500, px));
      root.style.setProperty('--sidebar-width', px + 'px');
      localStorage.setItem('zamin-sidebar-width', String(px));
    }

    function isSidebarCollapsed() {
      return localStorage.getItem('zamin-sidebar-collapsed') === 'true';
    }

    function setSidebarCollapsed(collapsed) {
      sidebar.classList.toggle('collapsed', collapsed);
      localStorage.setItem('zamin-sidebar-collapsed', String(collapsed));
      if (sidebarToggle) {
        sidebarToggle.title = collapsed ? 'Expand sidebar' : 'Collapse sidebar';
      }
    }

    // Restore sidebar state
    setSidebarWidth(getSidebarWidth());
    setSidebarCollapsed(isSidebarCollapsed());

    // Toggle collapse
    if (sidebarToggle) {
      sidebarToggle.addEventListener('click', function(e) {
        e.stopPropagation();
        setSidebarCollapsed(!sidebar.classList.contains('collapsed'));
      });
    }

    // Drag resize
    if (sidebarGrip) {
      var startX, startW;

      sidebarGrip.addEventListener('mousedown', function(e) {
        e.preventDefault();
        startX = e.clientX;
        startW = parseInt(getComputedStyle(root).getPropertyValue('--sidebar-width')) || getSidebarWidth();
        document.body.classList.add('sidebar-resizing');
        sidebarGrip.classList.add('active');
        sidebar.style.transition = 'none';
        var c = document.querySelector('.content');
        if (c) c.style.transition = 'none';
        var m = document.querySelector('.main-layout');
        if (m) m.style.transition = 'none';
      });

      document.addEventListener('mousemove', function(e) {
        if (!sidebarGrip.classList.contains('active')) return;
        var newW = startW + (e.clientX - startX);
        setSidebarWidth(newW);
      });

      document.addEventListener('mouseup', function() {
        if (!sidebarGrip.classList.contains('active')) return;
        document.body.classList.remove('sidebar-resizing');
        sidebarGrip.classList.remove('active');
        sidebar.style.transition = '';
        var c = document.querySelector('.content');
        if (c) c.style.transition = '';
        var m = document.querySelector('.main-layout');
        if (m) m.style.transition = '';
      });
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

    // === Language Dropdown ===
    const langBtn = document.getElementById('lang-btn');
    const langMenu = document.getElementById('lang-menu');
    const langOptions = document.querySelectorAll('.lang-option');

    function getCurrentLang() {
      var path = window.location.pathname;
      if (path.indexOf('/fa/') >= 0) return 'fa';
      if (path.indexOf('/ja/') >= 0) return 'ja';
      return 'en';
    }

    function langHref(targetLang) {
      var path = window.location.pathname;
      var dir = path.substring(0, path.lastIndexOf('/') + 1);
      dir = dir.replace(/\/(fa|ja)\//, '/');
      var page = path.substring(path.lastIndexOf('/') + 1) || 'index.html';
      if (targetLang === 'en') return dir + page;
      return dir + targetLang + '/' + page;
    }

    if (langBtn && langMenu) {
      const currentLang = getCurrentLang();
      langOptions.forEach(function(o) {
        if (o.getAttribute('data-lang') === currentLang) {
          o.classList.add('active');
        } else {
          o.classList.remove('active');
        }
      });
      langBtn.textContent = '\uD83C\uDF10 ' + currentLang.toUpperCase();

      langBtn.addEventListener('click', function(e) {
        e.stopPropagation();
        langMenu.classList.toggle('active');
      });

      document.addEventListener('click', function() {
        langMenu.classList.remove('active');
      });

      langOptions.forEach(function(opt) {
        opt.addEventListener('click', function(e) {
          e.preventDefault();
          window.location.href = langHref(opt.getAttribute('data-lang'));
        });
      });
    }

    // === GitHub Link ===
    const editBtn = document.getElementById('edit-link');
    if (editBtn) {
      var path = window.location.pathname;
      var page = path.substring(path.lastIndexOf('/') + 1) || 'index.html';
      var lang = getCurrentLang();
      var localeDir = lang !== 'en' ? lang + '/' : '';
      editBtn.href = 'https://github.com/young-developer90/zamin/edit/master/docs/' + localeDir + page;
    }
  });

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }
})();
