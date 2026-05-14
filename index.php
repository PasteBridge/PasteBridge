<!DOCTYPE html>
<script>
    //禁用“确认重新提交表单”
    window.history.replaceState(null, null, window.location.href);
</script>

<head>
    <meta name="viewport" content="width=device-width, initial-scale=0.7, minimum-scale=0.8, maximum-scale=2.0, user-scalable=yes" />
    <meta http-equiv="content-type" content="text/html;charset=utf-8">
    <title>PureCopy | 纯净复制</title>
    <link href="index.css" rel="stylesheet" type="text/css" />
    <link href="font-awesome-4.7.0\css\font-awesome.min.css" rel="stylesheet" type="text/css" />
    <link href="favicon.ico" rel="shortcut icon">
    <!-- <script type="text/javascript" src="https://pss.bdstatic.com/r/www/cache/static/protocol/https/jquery/jquery-1.10.2.min_65682a2.js"></script> -->
    <script type="text/javascript" src="js\jquery-3.6.0.min.js"></script>
    <script type="text/javascript" src="js\jquery-form.js"></script>
    <script type="text/javascript" src="js\jquery.cookie.js"></script>
    <script type="text/javascript" src="js\console.welcome.js"></script>
    <!-- <script src="http://libs.baidu.com/jquery/2.0.3/jquery.min.js"></script>     -->
    <!-- Compiled and minified CSS -->
        <link rel="stylesheet" href="materialize\css\materialize.css">

    <!-- Compiled and minified JavaScript -->
        <script src="materialize\js\materialize.js"></script>
    
        
</head>

<body>
    <div id="bg">
        <div id="bg-cover-cover"></div>
        <div id="bg-cover"></div>

    </div>

    <div id="main-content-box" class="z-depth-5">
        <!-- 移动端功能列表呼出按钮 -->
        <button class="waves-effect waves-light white z-depth-1" id="func-button" onclick="CtrlOn()" onmouseover="hovertip('打开功能列表')" onmouseout="hovertipoff()"><i class="fa fa-navicon"></i></button>
        
        <div id="data-list-box" class="z-depth-2">
            <div id="refresh-data-list">
                <svg class="circular" height="50" width="50">
                    <circle class="path" cx="25" cy="25.2" r="19.9" fill="none" stroke-width="4" stroke-miterlimit="10" />
                </svg>
            </div>
        </div> 

        

        <div id="introduction-box" class="z-depth-2">
            <div>
                <p>PureCopy，是一个纯净的网页剪贴板，可以快速地跨平台传输文字。</p>
                <p>你可以自由创建剪贴板，并在各个设备上打开以获取文字。</p>
                <p>虽然有密码保护剪贴板，但是并不保证文字存放在此处的安全性。</p>
                <p>要创建密码，在密码框中输入即可。</p>
                <p>要为已创建的剪贴板设定密码，在密码框中输入即可。</p>
                <p>请不要在剪贴板中传输机密信息。</p>
                <p>就这么多，祝你使用愉快。</p>
                <p><i>问询：pencilzyl@gmail.com</i></p>
                <a onclick="introductionOff()">关闭</a>
            </div>

        </div> 


        <div id="control-box">
            <div id="tool-bar">
                <div id="tool-bar-content">
                    <ul id="tool-bar-content-list">
                        <li class="tool-bar-content-list-item" id="cloud-sync-button">
                            <i class="fa fa-cloud-download"></i>
                            <span>云同步</span>
                        </li>
                        <li class="tool-bar-content-list-item" id="delete-all-button">
                            <i class="fa fa-trash-o"></i>
                            <span>删除全部</span>
                        </li>
                    </ul>
                </div>
                <button class="waves-effect waves-light white z-depth-1" id="exit-button" onclick="loginOn()" onmouseover="hovertip('退出')" onmouseout="hovertipoff()"><i class="fa fa-power-off"></i></button>
            </div> 
            <!-- <div id="favicon">
                <img id="favicon-logo" src="pic\favicon@0,1x.png">
                <img id="favicon-text" src="pic\logotext-cn@0,5x.png">
                
            </div> -->
            <div id="input-box-info">
                <span id="last-cloud-sync-input-box-info"></span>
                <span id="copyroom-name-input-box-info"></span>
                <span id="copyroom-number-box-info"></span>
            </div>
            <div id="input-box" >
                <textarea id="input-box-textarea" class="z-depth-1" placeholder="输入文本"></textarea>
            </div>
            <button class="waves-effect waves-light white z-depth-1" id="upload-button" onclick="sendClipBoardData()"><i class="fa fa-paper-plane"></i></button>
            
        </div> 

        <div id="login-box">
            <div id="favicon">
                <img id="favicon-logo" src="pic\favicon@0,1x.png">
                <img id="favicon-text" src="pic\logotext-cn@0,5x.png">
                
            </div>
            
            <div id="change-password-box">你想为这个无密码剪贴板添加一个密码吗？<br/>
                <button class="waves-effect waves-light white z-depth-1" id="change-password-button">是</button>
                <button class="waves-effect waves-light white z-depth-1" id="change-password-button-no">否</button></div>
            <div id="login-loading-box">加载中…</div>
            <div id="login-input-box" class="z-depth-1">
            <form id="login-form">
                <div class="input-field">
                    <input id="copyroom-input" type="text" class="" name="copyroom-name">
                    <label for="copyroom-input">剪贴板名</label>
                    <span id="copyroom-helpertext" class="helper-text" data-error="已被占用" data-success="可使用"></span>
                </div>
                <div class="input-field" id="copyroompassword-input-field">
                    <input id="copyroompassword-input" class="" type="text" name="copyroom-password">
                    <label for="copyroompassword-input">密码（可留空）</label>
                    <span id="copyroompassword-helpertext" class="helper-text" data-error="密码错误"></span>
                </div>
                <!-- <div class="input-field" id="copyroomtimelimit-input-field">
                    <input id="copyroomtimelimit-input" type="number" placeholder="1" name="copyroom-timelimit">
                    <label for="copyroomtimelimit-input">期限（天）</label>
                </div> -->
            </form>
            </div>
            <!-- 登录按钮 -->
            <button class="waves-effect waves-light white z-depth-1" id="login-button" onclick="copyRoomLogin()"><i class="fa fa-paper-plane"></i></button>
        </div> 


    </div> 



    <div id="footer">
    
        <span id="site-info">PureCopy © PencilCore 2022 - NOW</span>
        <br/>
        <span id="site-info"><a onclick="introductionOn()">介绍</a></span>
    </div>
    </div>

    <div id="tips" class="z-depth-1"></div>

    <script type="text/javascript" src="js\index.js"></script>

</body>

</html>
