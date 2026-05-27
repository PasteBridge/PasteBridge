document.addEventListener('DOMContentLoaded', () => {
    const background = document.querySelector('.background');
    let hue = 0;
    
    const animateHue = () => {
        hue = (hue + 0.5) % 360;
        background.style.filter = `hue-rotate(${hue}deg)`;
        requestAnimationFrame(animateHue);
    };
    
    animateHue();
});