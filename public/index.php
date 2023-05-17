<!DOCTYPE html>
<script>
    // 禁用“确认重新提交表单”
    window.history.replaceState(null, null, window.location.href);
</script>

<head>
    <meta name="viewport" content="width=device-width, initial-scale=0.7, minimum-scale=0.8, maximum-scale=2.0, user-scalable=yes"/>
    <meta http-equiv="content-type" content="text/html;charset=utf-8">
    <title>PureCopy | 纯净复制</title>
    <link href="public/assets/css/index.css" rel="stylesheet" type="text/css"/>
    <link href="public/assets/font-awesome/css/font-awesome.min.css" rel="stylesheet" type="text/css"/>
    <link href="public/assets/favicon.ico" rel="shortcut icon">
    <!-- Materialize:https://github.com/Dogfalo/materialize Code Copyright 2018 Materialize. Code released under the MIT license. -->
    <link rel="stylesheet" href="public/assets/materialize/css/materialize.min.css">
    <script src="public/assets/materialize/js/materialize.min.jss"></script>

    <link rel="stylesheet/less" type="text/css" href="public/assets/less/less.less" />


</head>

<body>
    <div id="bg">
        <div id="bg-cover"></div>

    </div>
    <div class="body"></div>
    <div id="main-content-box" class="z-depth-5">

        <div id="data-list-box" class="z-depth-2 left-box">
            <div id="refresh-data-list">
                <svg class="circular" height="50" width="50">
                    <circle class="path" cx="25" cy="25.2" r="19.9" fill="none" stroke-width="4" stroke-miterlimit="10"/>
                </svg>
            </div>
        </div>


        <div id="control-box" class="right-box">
            <div id="input-box">
                <textarea id="input-box-textarea" class="z-depth-1" placeholder="输入文本"></textarea>
            </div>
            <button class="waves-effect waves-light white z-depth-1" id="upload-button" onclick="sendClipBoardData()">
                <i class="fa fa-paper-plane"></i>
            </button>
        </div>


        <div id="login-box" class="right-box">
            <div id="favicon">
                <img id="favicon-logo" src="pic\favicon@0,1x.png">
                <img id="favicon-text" src="pic\logotext-cn@0,5x.png">

            </div>

            <div id="change-password-box">你想为这个无密码剪贴板添加一个密码吗？<br/>
                <button class="waves-effect waves-light white z-depth-1" id="change-password-button">是</button>
                <button class="waves-effect waves-light white z-depth-1" id="change-password-button-no">否</button>
            </div>
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
                </form>
                <!-- 登录按钮 -->
                <button class="waves-effect waves-light white z-depth-1" id="login-button" onclick="copyRoomLogin()">
                    <i class="fa fa-paper-plane"></i>
                </button>
            </div>

        </div>

    </div>
    <div id="footer">
        <span id="site-info">PureCopy © PencilCore 2022 - NOW</span>
        <br/>
        <span id="site-info">
            <a onclick="introductionOn()">介绍</a>
        </span>
    </div>
</body>
