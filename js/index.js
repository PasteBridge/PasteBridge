var option = {
    cursor_l_offset : 20,
    cursor_t_offset : 20
};

window.mobileCheck = function() {
    let check = false;
    (function(a){if(/(android|bb\d+|meego).+mobile|avantgo|bada\/|blackberry|blazer|compal|elaine|fennec|hiptop|iemobile|ip(hone|od)|iris|kindle|lge |maemo|midp|mmp|mobile.+firefox|netfront|opera m(ob|in)i|palm( os)?|phone|p(ixi|re)\/|plucker|pocket|psp|series(4|6)0|symbian|treo|up\.(browser|link)|vodafone|wap|windows ce|xda|xiino/i.test(a)||/1207|6310|6590|3gso|4thp|50[1-6]i|770s|802s|a wa|abac|ac(er|oo|s\-)|ai(ko|rn)|al(av|ca|co)|amoi|an(ex|ny|yw)|aptu|ar(ch|go)|as(te|us)|attw|au(di|\-m|r |s )|avan|be(ck|ll|nq)|bi(lb|rd)|bl(ac|az)|br(e|v)w|bumb|bw\-(n|u)|c55\/|capi|ccwa|cdm\-|cell|chtm|cldc|cmd\-|co(mp|nd)|craw|da(it|ll|ng)|dbte|dc\-s|devi|dica|dmob|do(c|p)o|ds(12|\-d)|el(49|ai)|em(l2|ul)|er(ic|k0)|esl8|ez([4-7]0|os|wa|ze)|fetc|fly(\-|_)|g1 u|g560|gene|gf\-5|g\-mo|go(\.w|od)|gr(ad|un)|haie|hcit|hd\-(m|p|t)|hei\-|hi(pt|ta)|hp( i|ip)|hs\-c|ht(c(\-| |_|a|g|p|s|t)|tp)|hu(aw|tc)|i\-(20|go|ma)|i230|iac( |\-|\/)|ibro|idea|ig01|ikom|im1k|inno|ipaq|iris|ja(t|v)a|jbro|jemu|jigs|kddi|keji|kgt( |\/)|klon|kpt |kwc\-|kyo(c|k)|le(no|xi)|lg( g|\/(k|l|u)|50|54|\-[a-w])|libw|lynx|m1\-w|m3ga|m50\/|ma(te|ui|xo)|mc(01|21|ca)|m\-cr|me(rc|ri)|mi(o8|oa|ts)|mmef|mo(01|02|bi|de|do|t(\-| |o|v)|zz)|mt(50|p1|v )|mwbp|mywa|n10[0-2]|n20[2-3]|n30(0|2)|n50(0|2|5)|n7(0(0|1)|10)|ne((c|m)\-|on|tf|wf|wg|wt)|nok(6|i)|nzph|o2im|op(ti|wv)|oran|owg1|p800|pan(a|d|t)|pdxg|pg(13|\-([1-8]|c))|phil|pire|pl(ay|uc)|pn\-2|po(ck|rt|se)|prox|psio|pt\-g|qa\-a|qc(07|12|21|32|60|\-[2-7]|i\-)|qtek|r380|r600|raks|rim9|ro(ve|zo)|s55\/|sa(ge|ma|mm|ms|ny|va)|sc(01|h\-|oo|p\-)|sdk\/|se(c(\-|0|1)|47|mc|nd|ri)|sgh\-|shar|sie(\-|m)|sk\-0|sl(45|id)|sm(al|ar|b3|it|t5)|so(ft|ny)|sp(01|h\-|v\-|v )|sy(01|mb)|t2(18|50)|t6(00|10|18)|ta(gt|lk)|tcl\-|tdg\-|tel(i|m)|tim\-|t\-mo|to(pl|sh)|ts(70|m\-|m3|m5)|tx\-9|up(\.b|g1|si)|utst|v400|v750|veri|vi(rg|te)|vk(40|5[0-3]|\-v)|vm40|voda|vulc|vx(52|53|60|61|70|80|81|83|85|98)|w3c(\-| )|webc|whit|wi(g |nc|nw)|wmlb|wonu|x700|yas\-|your|zeto|zte\-/i.test(a.substr(0,4))) check = true;})(navigator.userAgent||navigator.vendor||window.opera);
    return check;
  };

if (window.mobileCheck() == false){
    document.onmousemove = function (mousemoveevent){   
        mousemoveevent = mousemoveevent || window.mousemoveevent;   
        let cursorLeft = mousemoveevent.clientX;   
        let cursorTop = mousemoveevent.clientY;
        document.getElementById("tips").style.left=cursorLeft + option.cursor_l_offset +"px";
        document.getElementById("tips").style.top=cursorTop + option.cursor_t_offset +"px";
    } 
    function hovertip(str){ 
        document.getElementById("tips").innerHTML=str;
        document.getElementById("tips").style.opacity = 0.9;
        document.getElementById("tips").style.transform = "scaleY(1)";
    }
    function hovertipoff(){
        document.getElementById("tips").style.opacity = 0;
        document.getElementById("tips").style.transform = "scaleY(0.2)";
        console.log(1)
    }
}

function copyText(obj){         
    var textToCopy = $(obj).find(".data-list-item-content").eq(0).html().replace(/<br>/g,"\n");
    console.log(textToCopy)
    navigator.clipboard.writeText(textToCopy);                      
    hovertip("已复制");
}

{
    function clearDataList(){
        var data_list = document.getElementById("data-list");
        data_list.style.opacity = 0;  
        data_list.remove(); 
    }
    function refreshDataList(){
        document.getElementById("refresh-data-list").style.opacity="1";
        document.getElementById("refresh-data-list").style.zIndex="50"; 
        document.getElementById("refresh-data-list").style.height="100%"; 
        var data_list = document.getElementById("data-list");
        data_list.remove(); 
               
    }
    //z-index 5,10,20
    box = document.getElementById("introduction-box");
    ctrlbox = document.getElementById("control-box");
    loginbox = document.getElementById("login-box");
    changepasswordbox = document.getElementById("change-password-box");
    loginloadingbox = document.getElementById("login-loading-box");
    function introductionOn(){
        box.style.opacity = 1;
        box.style.zIndex = 50;
    }
    function introductionOff(){
        box.style.opacity = 0;
        box.style.zIndex = 5;
    }
    function loginOn(){
        clearDataList();
        loginLoadingOff();
        document.getElementById("data-list-box").style.filter = "blur(0px)"
        loginbox.style.opacity = 1;
        loginbox.style.zIndex = 10;
        controlOff()
    }
    function loginOff(){
        loginbox.style.opacity = 0;
        loginbox.style.zIndex = 5;
    }
    function changePassWordOn(){
        loginLoadingOff();
        changepasswordbox.style.opacity = 0.8;
        changepasswordbox.style.zIndex = 10;
        controlOff()
    }
    document.getElementById("change-password-button-no").onclick = changePassWordOff;
    
    function changePassWordOff(){
        changepasswordbox.style.opacity = 0;
        setTimeout(function(){
            changepasswordbox.style.zIndex = 0;
        },300)
    }

    var changepasswordboxhtml = changepasswordbox.innerHTML;
    function changePassWordOff(){
        changepasswordbox.style.opacity = 0;
        setTimeout(function(){
            changepasswordbox.style.zIndex = 0;
            changepasswordbox.innerHTML=changepasswordboxhtml;
            document.getElementById("change-password-button-no").onclick = changePassWordOff;
        },300)   
        
    }
    function loginLoadingOn(){
        refreshDataList;
        loginloadingbox.style.opacity = 0.8;
        loginloadingbox.style.zIndex = 10;
    }
    function loginLoadingOff(){
        loginloadingbox.style.opacity = 0;
        loginloadingbox.style.zIndex = 0;
    }
    loginbox.classList.add("zIndex40")
    function controlOn(){
        ctrlbox.style.opacity = 1;
        ctrlbox.style.zIndex = 10;
        loginbox.classList.remove("zIndex40")
        document.getElementById("func-button").style.opacity = 1
        if(window.screen.width< 975){
            ctrlbox.classList.add("opacity0")
        }
        loginOff()
    }
    function controlOff(){
        ctrlbox.style.opacity = 0;
        ctrlbox.style.zIndex = 5;
        loginbox.classList.add("zIndex40")
        ctrlbox.classList.remove("zIndex40")
        document.getElementById("func-button").style.opacity = 0
    }
    // 移动端呼出
    function CtrlOn(){
        ctrlbox.style.opacity = 1
        ctrlbox.classList.add("zIndex40")
        document.getElementById("func-button").onclick = CtrlOff
        document.getElementById("data-list-box").style.filter = "blur(10px)"
        ctrlbox.classList.remove("opacity0")
        // document.getElementById("data-list-box").style.filter = "brightness(0.8)"
    }
    function CtrlOff(){
        ctrlbox.style.opacity = 0
        ctrlbox.classList.remove("zIndex40")
        document.getElementById("func-button").onclick = CtrlOn
        document.getElementById("data-list-box").style.filter = "blur(0px)"
        ctrlbox.classList.add("opacity0")
        // document.getElementById("data-list-box").style.filter = "brightness(1)"
    }
}

document.getElementById("copyroompassword-input").onclick= function(){
    this.classList.remove("invalid");
}
document.getElementById("copyroom-input").onclick= function(){
    this.classList.remove("invalid");
}

document.getElementById("login-button").onclick = copyRoomLogin;
function copyRoomLogin(){ 
    loginLoadingOn();
    var loginInData = new FormData(document.getElementById("login-form"));
    $.ajax({
            cache: false,
            dataType:"text",
            contentType : false,
            processData : false,
            url: "copyroomlogin.php",
            type: "post",
            data:loginInData,
            success: function(resultjson) {
                var result = JSON.parse(resultjson);
                console.log()
                if(result[0].password_change_require == true && document.getElementById("copyroompassword-input").value != ""){
                    if(result[0].admin == 1){
                        loginLoadingOff();
                        changepasswordbox.innerHTML="管理员指定不可设置密码<br/><button class='waves-effect waves-light white z-depth-1' id='change-password-button-no'>是</button>";
                        document.getElementById("change-password-button-no").onclick = changePassWordOff;
                    }
                    changePassWordOn();
                    document.getElementById("change-password-button").onclick= function changePassword(){
                        loginLoadingOn();
                        $.ajax({
                            url: "changepassword.php",
                            type: "post",
                            dataType: "text",
                            data: {
                                "token": 1,
                                "copyroom-name": loginInData.get("copyroom-name"),
                                "copyroom-password": loginInData.get("copyroom-password"),
                            },
                            success: function(resultjson) {                     
                                var result = JSON.parse(resultjson);
                                console.log(result);
                                changePassWordOff();
                                copyRoomLogin();
                                loginLoadingOff();
                            }
                        })
                    }
                }else if(result[0].password_verify == true){
                    //进入剪贴板
                    getClipBoardData();
                    document.getElementById("copyroompassword-input").classList.remove("invalid");
                    document.getElementById("copyroom-input").classList.remove("invalid"); 
                    controlOn(); 
                    loginLoadingOff();
                }else if(result[0].copyroom_exists == true && result[0].password_verify == false){
                    document.getElementById("copyroom-input").classList.add("invalid"); 
                    document.getElementById("copyroompassword-input").classList.add("invalid"); 
                    loginLoadingOff();
                }else if(result[0].password_verify == false){
                    document.getElementById("copyroompassword-input").classList.add("invalid");
                    loginLoadingOff();
                }
            }
        })
}


document.getElementById("copyroom-input").oninput = function copyRoomCheck(){
    var copyRoomName = document.getElementById("copyroom-input").value;
    if(copyRoomName != ""){
    $.ajax({
            url: "copyroomlogin.php",
            type: "post",
            dataType: "text",
            data: {
                "copyroom-name":copyRoomName,
                "applicability-check":true
            },
            success: function(resultjson) {
                var result = JSON.parse(resultjson);
                console.log()
                if(result[0].copyroom_exists == false){
                    document.getElementById("copyroom-input").classList.add("valid"); 
                    document.getElementById("copyroom-input").classList.remove("invalid"); 
                }else{
                    //进入剪贴板
                    document.getElementById("copyroom-input").classList.remove("valid"); 
                    document.getElementById("copyroom-input").classList.add("invalid"); 
                }
            }
    })}else{
        document.getElementById("copyroom-input").classList.remove("invalid");
        document.getElementById("copyroom-input").classList.remove("valid"); 
    }
}

document.getElementById("refresh-data-list").style.opacity="0";       
document.getElementById("refresh-data-list").style.zIndex="0"; 
document.getElementById("refresh-data-list").style.height="0"; 

document.getElementById("cloud-sync-button").onclick = cloudSync;

function cloudSync(){
    refreshDataList();
    setTimeout(function(){
        document.getElementById("refresh-data-list").style.opacity="0";       
        document.getElementById("refresh-data-list").style.zIndex="0"; 
        document.getElementById("refresh-data-list").style.height="0"; 
    },600)
    setTimeout(function(){
        getClipBoardData();
    },500)
}

function getClipBoardData(){
    var copyroom_name = $.cookie("copyroom_name_latest");
    var data_list = document.createElement("ul");
    data_list.id="data-list"
    document.getElementById("data-list-box").appendChild(data_list);
    $.ajax({
            url: "getclipboarddata.php",
            type: "post",
            dataType: "text",
            data: {
                "token": 1,
                "copyroom-name": copyroom_name,
            },
            success: function(resultjson) {                     
                var result = JSON.parse(resultjson);
                console.log(result);
                var resultnum = result[0].clipboard_data.length;
                var resultcontent = [];
                for (i = 0; i < resultnum; i++) {

                    resultcontent.push(result[0].clipboard_data[i].clipboarddata_copyroom);
                    result_li = document.createElement("li");
                    result_li.classList.add("data-list-item","waves-effect","waves-light");
                    if (i == 0){
                        result_li.classList.add("data-list-item-newest");
                    }
                    result_li_p = document.createElement("p");
                    result_li_p.classList.add("data-list-item-content");
                    
                    result_li_p.innerHTML = result[0].clipboard_data[i].clipboarddata_content.replace(/\n/g,"<br/>");
                    result_li_span = document.createElement("span");
                    result_li_span.classList.add("data-list-item-info");
                    result_li_span_time = document.createElement("span");
                    result_li_span_time.classList.add("data-list-item-info-time");

                    result_li_span_time.innerHTML=getLocalTime(result[0].clipboard_data[i].clipboarddata_createtime);

                    result_li.setAttribute("onclick","copyText(this)");
                    result_li.setAttribute("onmouseover","hovertip('单击复制')");
                    result_li.setAttribute("onmouseout","hovertipoff()");

                    result_li.appendChild(result_li_p);
                    result_li.appendChild(result_li_span);
                    result_li_span.appendChild(result_li_span_time);
                    data_list.appendChild(result_li);
                }
                data_count = document.createElement("div");
                data_count.setAttribute("id","data-count-sign");
                data_count.innerHTML = "共" + resultnum + "条结果";
                data_list.appendChild(data_count);
                data_list.style.opacity = 1;
                
                if(result[0].clipboard_data.length != 0){
                    document.getElementById("last-cloud-sync-input-box-info").innerHTML="最后编辑时间：" + getLocalTime(result[0].clipboard_data[0].clipboarddata_createtime) + "</br>";
                document.getElementById("copyroom-name-input-box-info").innerHTML="剪贴板名称：" + copyroom_name + "</br>";;
                document.getElementById("copyroom-number-box-info").innerHTML="共计 " + result[0].copyroom_number[0][0] + " 条项目" + "</br>";;
                $.cookie("last-sync-time",result[0].clipboard_data[0].clipboarddata_createtime);
                }
                
                

            }
        })
    
}



document.getElementById("upload-button").onclick = function sendClipBoardData(){
    var textarea_send = document.getElementById("input-box-textarea");
    var textToSend = textarea_send.value;
    // textToSend = textToSend.replace('\r\n', '\n');
    $.ajax({
            url: "sendclipboarddata.php",
            type: "post",
            dataType: "text",
            data: {
                "token": 1,
                "copyroom-name":$.cookie("copyroom_name_latest"),
                "text_to_send": textToSend,
            },
            success: function(resultjson) {
                var result = JSON.parse(resultjson);
                console.log(result);
                refreshDataList();
                setTimeout(function(){
                    document.getElementById("refresh-data-list").style.opacity="0";       
                    document.getElementById("refresh-data-list").style.zIndex="0"; 
                    document.getElementById("refresh-data-list").style.height="0"; 
                },600)
                setTimeout(function(){
                    getClipBoardData();
                },500) 
        }})
}

document.getElementById("delete-all-button").onclick = deleteClipBoardData;

function deleteClipBoardData(){
    $.ajax({
            url: "deleteclipboarddata.php",
            type: "post",
            dataType: "text",
            data: {
                "token": 1,
                "copyroom-name":$.cookie("copyroom_name_latest"),

            },
            success: function(resultjson) {
                var result = JSON.parse(resultjson);
                console.log(result);
                refreshDataList();
                setTimeout(function(){
                    document.getElementById("refresh-data-list").style.opacity="0";       
                    document.getElementById("refresh-data-list").style.zIndex="0"; 
                    document.getElementById("refresh-data-list").style.height="0"; 
                },600)
                setTimeout(function(){
                    getClipBoardData();
                },500)
        }})
}

function getLocalTime(nS) {  
    return new Date(parseInt(nS) * 1000).toLocaleString().replace(/:\d{1,2}$/,' ');  
}
   


function getTime(nowDate) {  
console.log(nowDate)
function getNow(s) {
	return s < 10 ? '0' + s : s;
};
	var myDate = new Date(nowDate);
	var year = myDate.getFullYear();        //获取当前年
	var month = myDate.getMonth() + 1;   	//获取当前月
	var date = myDate.getDate();            //获取当前日
	var h = myDate.getHours();              //获取当前小时数(0-23)
	var m = myDate.getMinutes();         	//获取当前分钟数(0-59)
	var s = myDate.getSeconds();
	var now = 	year + '-' + 
				getNow(month) + "-" + 
				getNow(date) + " " + 
				getNow(h) + ':' + 
				getNow(m) + ":" + 
				getNow(s);
	return now;
}

