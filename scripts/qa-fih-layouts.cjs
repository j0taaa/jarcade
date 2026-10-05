// Optional responsive and idle-loop checks. Requires Playwright and a web build.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
(async()=>{
 const b=await chromium.launch({executablePath:process.env.JARCADE_CHROME || undefined,headless:true,
  args:['--no-sandbox','--enable-unsafe-swiftshader','--disable-backgrounding-occluded-windows','--disable-background-timer-throttling']});
 const errors=[];
 for(const [width,height] of [[280,360],[320,480],[390,844],[568,320],[844,390],[768,1024],[1280,900]]){
  const c=await b.newContext({viewport:{width,height},deviceScaleFactor:1,hasTouch:true,isMobile:width<600});
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const tap=async(x,y)=>{await p.touchscreen.tap(x,y);await p.waitForTimeout(90);};
  const wait=t=>p.waitForFunction(t=>document.querySelector('#glcanvas').getAttribute('aria-label').includes(t),t);
  const shot=name=>p.screenshot({path:`/tmp/jarcade-${width}x${height}-${name}.png`});
  const margin=width<360?16:width<600?20:40, content=Math.min(width-margin*2,1040);
  const landscape=(width>height*1.3&&height<620)||(height<360&&width>=360);
  const cols=content>=960||(landscape&&content>=480)?4:content>=600||(landscape&&content>=480)?3:2;
  const gap=cols===2?12:20, cw=(content-gap*(cols-1))/cols, rows=Math.ceil(4/cols);
  const top=height<500?132:height<640?208:238;
  const ih=Math.min(cw*.88,Math.max(44,(height-top-24-gap*(rows-1))/rows-60));
  const card=index=>[(width-content)/2+(index%cols)*(cw+gap)+cw/2,top+Math.floor(index/cols)*(ih+60+gap)+(ih+60)/2];
  const wide=height<520&&width>height*1.3;
  const toolsW=Math.min(width-24,570),toolsX=(width-toolsW)/2;
  const tool=(i,count)=>wide?[width-116,146+i*(height-142)/count]:[toolsX+toolsW/count*(i+.5),height-75];
  const statsW=width<360?width-76:Math.min(Math.max(width-90,200),310,width-64);
  const statsX=width<360?64:(width-statsW)/2;
  const room=async(i,name)=>{await tap(statsX+statsW/5*(i+.5),28);await wait(`Fih. ${name}`);};
  await p.goto(process.env.JARCADE_QA_URL || 'http://127.0.0.1:8080/');await wait('Select Snake');await shot('home');
  await tap(...card(2));await wait('Fih. Kitchen');await shot('kitchen');
  await tap(...tool(0,3));await wait('Pantry');await shot('pantry');await p.keyboard.press('Escape');
  await tap(...tool(2,3));await wait('Food shop');await shot('food-shop');await p.keyboard.press('Escape');
  await room(3,'Bedroom');await tap(...tool(1,2));await wait('Wardrobe');await shot('wardrobe');await p.keyboard.press('Escape');
  await room(2,'Playroom');await tap(...tool(0,2));await wait('Mini-games');await shot('games');await p.keyboard.press('Escape');
  await room(4,'Clinic');await tap(...tool(1,2));await wait('Potion shop');await shot('potions');await p.keyboard.press('Escape');
  await tap(34,34);await wait('Select Snake');await tap(...card(0));await wait('Snake.');await shot('snake');
  await tap((width-content)/2+22,42);await wait('Select Snake');
  await tap(...card(1));await wait('Choose board size');await shot('mines-setup');
  const short=height<500, minesWide=short&&Math.min(width-40,560)>=480;
  await tap(width/2,(minesWide?228:short?Math.min(height-60,312):400)+26);
  await wait('Reveal mode');await shot('mines-board');
  await tap(30,26);await wait('Select Snake');
  await tap((width-content)/2+content-22,height<540?26:42);await wait('Settings.');await shot('settings');
  // Check an actual resize while Fih remains open, including the backing buffer.
  await tap((width-content)/2+content-22,height<540?26:42);await wait('Select Snake');await tap(...card(2));await wait('Fih. Kitchen');
  await p.setViewportSize({width:height,height:width});await p.waitForTimeout(120);await shot('rotated');
  assert.equal(await p.evaluate(()=>document.querySelector('#glcanvas').width),height);
  await c.close();
 }
 const c=await b.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});
 const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
 await p.addInitScript(()=>{localStorage.setItem('jarcade.settings.v1','3 1 0 0 1');window.renderedFrames=0;
  const raf=requestAnimationFrame.bind(window);window.requestAnimationFrame=fn=>raf(t=>{renderedFrames++;fn(t);});});
 await p.goto(process.env.JARCADE_QA_URL || 'http://127.0.0.1:8080/');
 await p.waitForFunction(()=>document.querySelector('#glcanvas').getAttribute('aria-label').includes('Select Snake'));
 await p.touchscreen.tap(100,520);await p.waitForTimeout(1500);
 const idle=async(message)=>{let n=await p.evaluate(()=>renderedFrames);await p.waitForTimeout(350);
  assert.equal(await p.evaluate(()=>renderedFrames),n,message);};
 await idle('saver FPS counter wakes idle fish');
 const bytes=await p.screenshot({path:'/tmp/fih-saver.png'});
 const pixel=spawnSync('python3',['-c','import sys,io;from PIL import Image;im=Image.open(io.BytesIO(sys.stdin.buffer.read()));print(im.getpixel((20,300)))'],{input:bytes});
 assert(pixel.stdout.toString().includes('(0, 0, 0)'),'saver background is not black');
 const cd=await c.newCDPSession(p);
 await cd.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:310,y:330,id:1}]});await p.waitForTimeout(150);
 await idle('stationary held finger wakes saver');await cd.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
 await p.touchscreen.tap(74,770);await p.waitForTimeout(180);await idle('saver menu redraws');
 await c.close();assert.deepEqual(errors,[]);
 console.log('PASS: seven viewport sizes, all arcade screens and Fih menus, rotation, true-black saver, idle FPS/menu/held-finger scheduling.');await b.close();
})().catch(e=>{console.error(e);process.exit(1)});
