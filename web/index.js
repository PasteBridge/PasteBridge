document.addEventListener('DOMContentLoaded', function() {
    'use strict';

    // ===== 底部导航激活状态 =====
    var navItems = document.querySelectorAll('.nav-item');
    var sections = document.querySelectorAll('section[id]');

    function updateActiveNav() {
        var scrollY = window.scrollY + window.innerHeight / 3;

        sections.forEach(function(section) {
            var sectionTop = section.offsetTop;
            var sectionHeight = section.offsetHeight;
            var sectionId = section.getAttribute('id');

            if (scrollY >= sectionTop && scrollY < sectionTop + sectionHeight) {
                navItems.forEach(function(item) {
                    item.classList.remove('active');
                    if (item.getAttribute('href') === '#' + sectionId) {
                        item.classList.add('active');
                    }
                });
            }
        });

        if (window.scrollY < window.innerHeight / 2) {
            navItems.forEach(function(item) {
                item.classList.remove('active');
                if (item.getAttribute('href') === '#hero') {
                    item.classList.add('active');
                }
            });
        }
    }

    window.addEventListener('scroll', updateActiveNav);
    updateActiveNav();

    // ===== 平滑滚动 =====
    navItems.forEach(function(item) {
        item.addEventListener('click', function(e) {
            var href = this.getAttribute('href');
            if (href && href.charAt(0) === '#') {
                e.preventDefault();
                var target = document.querySelector(href);
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                }
            }
        });
    });

    // ===== 滚动入场动画 =====
    var animatedElements = document.querySelectorAll(
        '.feature-item, .tech-card, .platform-card, .stat-card, .section-headline, .hero-content, .hero-product-card'
    );

    // Hero 内容立即显示
    var heroContent = document.querySelector('.hero-content');
    var heroCard = document.querySelector('.hero-product-card');
    if (heroContent) {
        heroContent.style.opacity = '1';
        heroContent.style.transform = 'translateY(0)';
    }
    if (heroCard) {
        heroCard.style.opacity = '1';
        heroCard.style.transform = 'translateY(0)';
        heroCard.style.transition = 'opacity 0.8s ease 0.2s, transform 0.8s ease 0.2s';
    }

    // 非 Hero 元素初始隐藏
    animatedElements.forEach(function(el) {
        if (!el.classList.contains('hero-content') && !el.classList.contains('hero-product-card')) {
            el.style.opacity = '0';
            el.style.transform = 'translateY(24px)';
            el.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        }
    });

    var observer = new IntersectionObserver(function(entries) {
        entries.forEach(function(entry) {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '';
                entry.target.style.transform = '';
                observer.unobserve(entry.target);
            }
        });
    }, {
        threshold: 0.08,
        rootMargin: '0px 0px -40px 0px'
    });

    animatedElements.forEach(function(el) {
        observer.observe(el);
    });

    // ===== 平台卡片点击效果 =====
    var platformCards = document.querySelectorAll('.platform-card');
    platformCards.forEach(function(card) {
        card.addEventListener('click', function(e) {
            e.preventDefault();
            var platformEl = this.querySelector('.platform-name');
            var platform = platformEl ? platformEl.textContent : '';
            alert('\u5373\u5c06\u4e0b\u8f7d PasteBridge for ' + platform + '...\n\n\uff08\u6f14\u793a\u6548\u679c\uff0c\u5b9e\u9645\u4e0b\u8f7d\u94fe\u63a5\u5f85\u914d\u7f6e\uff09');
        });
    });

    // ===== 顶部导航栏滚动变深 =====
    var topNav = document.querySelector('.top-nav');
    if (topNav) {
        window.addEventListener('scroll', function() {
            if (window.scrollY > 100) {
                topNav.style.background = 'rgba(0, 0, 0, 0.8)';
            } else {
                topNav.style.background = '';
            }
        });
    }
});