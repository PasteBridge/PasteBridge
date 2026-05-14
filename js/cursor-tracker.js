function cursorTracker(trackobj_id,parameter){
    let trackObj = document.getElementById(trackobj_id);
    document.onmousemove = function(mousemoveevent){
    mousemoveevent = mousemoveevent || window.mousemoveevent;
    let left = (mousemoveevent.clientX - (window.innerWidth / 2) )/parameter;
    let top = (mousemoveevent.clientY - (window.innerWidth / 2) )/parameter;
    trackObj.style.top = top + "px";
    trackObj.style.left = left + "px";
}}