document.addEventListener('DOMContentLoaded', function() {
    // ===== 背景彩虹动画 =====
    const bgLayer = document.querySelector('.bg-layer');
    const bgBlur = document.querySelector('.bg-blur');
    let hue = 0;
    
    function animateBackground() {
        hue = (hue + 0.15) % 100;
        const pos = hue;
        if (bgLayer) {
            bgLayer.style.backgroundPosition = `${pos}% 50%`;
        }
        if (bgBlur) {
            bgBlur.style.backgroundPosition = `${pos}% 50%`;
        }
        requestAnimationFrame(animateBackground);
    }
    
    if (bgLayer) {
        animateBackground();
    }
    
    // ===== 底部导航激活状态 =====
    const navItems = document.querySelectorAll('.nav-item');
    const sections = document.querySelectorAll('section[id]');
    
    function updateActiveNav() {
        const scrollY = window.scrollY + window.innerHeight / 3;
        
        sections.forEach(section => {
            const sectionTop = section.offsetTop;
            const sectionHeight = section.offsetHeight;
            const sectionId = section.getAttribute('id');
            
            if (scrollY >= sectionTop && scrollY < sectionTop + sectionHeight) {
                navItems.forEach(item => {
                    item.classList.remove('active');
                    if (item.getAttribute('href') === `#${sectionId}`) {
                        item.classList.add('active');
                    }
                });
            }
        });
        
        // 首页检测
        if (window.scrollY < window.innerHeight / 2) {
            navItems.forEach(item => {
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
    navItems.forEach(item => {
        item.addEventListener('click', function(e) {
            const href = this.getAttribute('href');
            if (href && href.startsWith('#')) {
                e.preventDefault();
                const target = document.querySelector(href);
                if (target) {
                    target.scrollIntoView({
                        behavior: 'smooth',
                        block: 'start'
                    });
                }
            }
        });
    });
    
    // ===== 滚动淡入动画 =====
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };
    
    const observer = new IntersectionObserver(function(entries) {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);
    
    // Hero 动画
    const heroBanner = document.querySelector('.hero-banner');
    if (heroBanner) {
        heroBanner.style.opacity = '0';
        heroBanner.style.transition = 'opacity 0.8s ease';
        
        setTimeout(() => {
            heroBanner.style.opacity = '1';
        }, 100);
    }
    
    // 动画元素
    const animatedElements = document.querySelectorAll('.feature-item, .tech-card, .platform-card');
    animatedElements.forEach((el, index) => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(30px)';
        el.style.transition = `opacity 0.6s ease ${index * 0.1}s, transform 0.6s ease ${index * 0.1}s`;
        observer.observe(el);
    });
    
    // ===== 平台卡片点击效果 =====
    const platformCards = document.querySelectorAll('.platform-card');
    platformCards.forEach(card => {
        card.addEventListener('click', function(e) {
            e.preventDefault();
            const platform = this.querySelector('.platform-name').textContent;
            alert(`即将下载 PasteBridge for ${platform}...\n\n（演示效果，实际下载链接待配置）`);
        });
    });
    
    // ===== Hero Banner 3D 悬浮效果 =====
    if (heroBanner && window.innerWidth > 768) {
        const heroImg = heroBanner.querySelector('.hero-banner-img');
        if (heroImg) {
            heroBanner.addEventListener('mousemove', function(e) {
                const rect = heroImg.getBoundingClientRect();
                const x = e.clientX - rect.left;
                const y = e.clientY - rect.top;
                const centerX = rect.width / 2;
                const centerY = rect.height / 2;
                const rotateX = (y - centerY) / 40;
                const rotateY = (centerX - x) / 40;
                
                heroImg.style.transform = `perspective(1000px) rotateX(${rotateX}deg) rotateY(${rotateY}deg)`;
                heroImg.style.transition = 'none';
            });
            
            heroBanner.addEventListener('mouseleave', function() {
                heroImg.style.transform = 'perspective(1000px) rotateX(0) rotateY(0)';
                heroImg.style.transition = 'transform 0.5s ease';
            });
        }
    }
});
