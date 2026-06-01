document.addEventListener('DOMContentLoaded', function() {
    const background = document.querySelector('.background');
    let hue = 0;
    
    function animateHue() {
        hue = (hue + 0.15) % 360;
        background.style.filter = `hue-rotate(${hue}deg) brightness(1)`;
        requestAnimationFrame(animateHue);
    }
    
    animateHue();
    
    const navbar = document.querySelector('.navbar');
    let lastScroll = 0;
    
    window.addEventListener('scroll', function() {
        const currentScroll = window.pageYOffset;
        
        if (currentScroll > 50) {
            navbar.style.boxShadow = '0 1px 2px 0 rgba(60,64,67,0.3), 0 1px 3px 1px rgba(60,64,67,0.15)';
        } else {
            navbar.style.boxShadow = 'none';
        }
        
        lastScroll = currentScroll;
    });
    
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
    
    const animatedElements = document.querySelectorAll('.feature-card, .tech-item, .download-card');
    animatedElements.forEach(el => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = 'opacity 0.4s ease, transform 0.4s ease';
        observer.observe(el);
    });
    
    const smoothScrollLinks = document.querySelectorAll('a[href^="#"]');
    smoothScrollLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            e.preventDefault();
            const targetId = this.getAttribute('href');
            const targetElement = document.querySelector(targetId);
            
            if (targetElement) {
                const navHeight = navbar.offsetHeight;
                const targetPosition = targetElement.offsetTop - navHeight;
                
                window.scrollTo({
                    top: targetPosition,
                    behavior: 'smooth'
                });
            }
        });
    });
    
    const downloadButtons = document.querySelectorAll('.btn-download');
    downloadButtons.forEach(button => {
        button.addEventListener('click', function(e) {
            e.preventDefault();
            const platform = this.closest('.download-card').querySelector('.download-platform').textContent;
            alert(`即将开始下载 PasteBridge for ${platform}...\n\n（这是演示效果，实际下载链接需要配置）`);
        });
    });
    
    const heroContent = document.querySelector('.hero-content');
    if (heroContent) {
        heroContent.style.opacity = '0';
        heroContent.style.transform = 'translateX(-20px)';
        
        setTimeout(() => {
            heroContent.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
            heroContent.style.opacity = '1';
            heroContent.style.transform = 'translateX(0)';
        }, 300);
    }
    
    const heroVisual = document.querySelector('.hero-visual');
    if (heroVisual) {
        heroVisual.style.opacity = '0';
        heroVisual.style.transform = 'translateX(20px)';
        
        setTimeout(() => {
            heroVisual.style.transition = 'opacity 0.6s ease 0.2s, transform 0.6s ease 0.2s';
            heroVisual.style.opacity = '1';
            heroVisual.style.transform = 'translateX(0)';
        }, 300);
    }
    
    const canvas = document.getElementById('bridgeCanvas');
    const hero = document.querySelector('.hero');
    
    if (!canvas || !hero) {
        console.error('Canvas or hero not found');
    } else {
        const ctx = canvas.getContext('2d');
        let animationProgress = 0;
        
        function resizeCanvas() {
            canvas.width = hero.offsetWidth;
            canvas.height = hero.offsetHeight;
            animateBridge();
        }
        
        function drawBridge(progress) {
            ctx.clearRect(0, 0, canvas.width, canvas.height);
            
            const padding = canvas.width * 0.05;
            const startX = padding;
            const endX = canvas.width - padding;
            const startY = canvas.height * 0.7;
            const endY = canvas.height * 0.7;
            const curveHeight = canvas.height * 0.4;
            
            const controlPoint1X = startX + (endX - startX) * 0.25;
            const controlPoint1Y = startY - curveHeight;
            const controlPoint2X = startX + (endX - startX) * 0.75;
            const controlPoint2Y = startY - curveHeight;
            
            const offscreenCanvas = document.createElement('canvas');
            offscreenCanvas.width = canvas.width;
            offscreenCanvas.height = canvas.height;
            const offCtx = offscreenCanvas.getContext('2d');
            
            const gradient = offCtx.createLinearGradient(startX, 0, endX, 0);
            gradient.addColorStop(0, 'rgba(66, 133, 244, 0.8)');
            gradient.addColorStop(0.5, 'rgba(52, 168, 83, 0.9)');
            gradient.addColorStop(1, 'rgba(66, 133, 244, 0.8)');
            
            offCtx.shadowColor = 'rgba(66, 133, 244, 0.6)';
            offCtx.shadowBlur = 15;
            offCtx.strokeStyle = gradient;
            offCtx.lineWidth = 3;
            offCtx.lineCap = 'round';
            
            offCtx.beginPath();
            offCtx.moveTo(startX, startY);
            offCtx.bezierCurveTo(
                controlPoint1X, controlPoint1Y,
                controlPoint2X, controlPoint2Y,
                endX, endY
            );
            offCtx.stroke();
            
            offCtx.shadowBlur = 0;
            
            offCtx.beginPath();
            offCtx.setLineDash([10, 5]);
            offCtx.strokeStyle = 'rgba(255, 255, 255, 0.3)';
            offCtx.lineWidth = 1;
            
            offCtx.beginPath();
            offCtx.moveTo(startX, startY);
            offCtx.bezierCurveTo(
                controlPoint1X, controlPoint1Y,
                controlPoint2X, controlPoint2Y,
                endX, endY
            );
            offCtx.stroke();
            offCtx.setLineDash([]);
            
            const nodeRadius = 6;
            const gradient2 = offCtx.createRadialGradient(
                startX, startY, 0,
                startX, startY, nodeRadius * 2
            );
            gradient2.addColorStop(0, 'rgba(66, 133, 244, 1)');
            gradient2.addColorStop(1, 'rgba(66, 133, 244, 0)');
            
            offCtx.beginPath();
            offCtx.arc(startX, startY, nodeRadius * 2, 0, Math.PI * 2);
            offCtx.fillStyle = gradient2;
            offCtx.fill();
            
            offCtx.beginPath();
            offCtx.arc(startX, startY, nodeRadius, 0, Math.PI * 2);
            offCtx.fillStyle = '#4285F4';
            offCtx.fill();
            
            const nodeGradient = offCtx.createRadialGradient(
                endX, endY, 0,
                endX, endY, nodeRadius * 2
            );
            nodeGradient.addColorStop(0, 'rgba(66, 133, 244, 1)');
            nodeGradient.addColorStop(1, 'rgba(66, 133, 244, 0)');
            
            offCtx.beginPath();
            offCtx.arc(endX, endY, nodeRadius * 2, 0, Math.PI * 2);
            offCtx.fillStyle = nodeGradient;
            offCtx.fill();
            
            offCtx.beginPath();
            offCtx.arc(endX, endY, nodeRadius, 0, Math.PI * 2);
            offCtx.fillStyle = '#4285F4';
            offCtx.fill();
            
            ctx.drawImage(offscreenCanvas, 0, 0);
            
            ctx.globalCompositeOperation = 'destination-out';
            
            const safeProgress = Math.max(0, Math.min(1, progress));
            
            if (safeProgress > 0) {
                const revealGradient = ctx.createLinearGradient(startX, 0, endX, 0);
                
                if (safeProgress < 0.01) {
                    revealGradient.addColorStop(0, 'rgba(0, 0, 0, 1)');
                    revealGradient.addColorStop(1, 'rgba(0, 0, 0, 1)');
                } else if (safeProgress > 0.99) {
                    revealGradient.addColorStop(0, 'rgba(0, 0, 0, 0)');
                    revealGradient.addColorStop(1, 'rgba(0, 0, 0, 0)');
                } else {
                    revealGradient.addColorStop(0, 'rgba(0, 0, 0, 1)');
                    revealGradient.addColorStop(Math.max(0.01, safeProgress - 0.1), 'rgba(0, 0, 0, 1)');
                    revealGradient.addColorStop(safeProgress, 'rgba(0, 0, 0, 0)');
                    revealGradient.addColorStop(1, 'rgba(0, 0, 0, 0)');
                }
                
                ctx.fillStyle = revealGradient;
                ctx.fillRect(startX, startY - 50, endX - startX + 50, 100);
            }
            
            ctx.globalCompositeOperation = 'source-over';
        }
        
        function animateBridge() {
            const duration = 2000;
            const startTime = performance.now();
            
            function animate(currentTime) {
                const elapsed = currentTime - startTime;
                animationProgress = Math.min(elapsed / duration, 1);
                
                const easeOutCubic = 1 - Math.pow(1 - animationProgress, 3);
                
                drawBridge(easeOutCubic);
                
                if (animationProgress < 1) {
                    requestAnimationFrame(animate);
                }
            }
            
            requestAnimationFrame(animate);
        }
        
        window.addEventListener('resize', resizeCanvas);
        resizeCanvas();
    }
});
