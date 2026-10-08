exports.play=(w,h,sandwich=false)=>{
  const side=w>=850||w>=480&&h<550;
  let board,pad,actions;
  if(side){const pw=h<450?194:256,size=Math.max(96,Math.min(h-112,w-pw-64,630)),x=(w-size-pw-28)/2,top=h<450?64:100;board={x,y:top,w:size};pad={x:x+size+28,y:top,w:pw};actions=h<450?{x:(w-340)/2,y:h-48,w:340,h:44}:{x:pad.x-4,y:top+pw+12,w:pw+8,h:44};}
  else{const pw=Math.min(w-32,h<700?194:288),size=Math.max(96,Math.min(w-24,h-72-pw-98,600));board={x:(w-size)/2,y:72,w:size};pad={x:(w-pw)/2,y:72+size+16,w:pw};const aw=Math.min(w-24,340);actions={x:(w-aw)/2,y:pad.y+pw+10,w:aw,h:44};}
  if(sandwich){const m=Math.max(14,Math.min(48,board.w/9*.75));board={x:board.x+m,y:board.y+m,w:board.w-2*m};}
  const unit=(pad.w-18)/4;
  return {cell:i=>[board.x+(i%9+.5)*board.w/9,board.y+(Math.floor(i/9)+.5)*board.w/9],number:d=>[pad.x+((d-1)%3)*(unit+6)+unit/2,pad.y+Math.floor((d-1)/3)*(unit+6)+unit/2],tool:i=>[pad.x+3*(unit+6)+unit/2,pad.y+i*(unit+6)+unit/2],action:i=>[actions.x+(i+.5)*actions.w/6,actions.y+actions.h/2],erase:[pad.x+unit/2,pad.y+3*(unit+6)+unit/2],candidates:[pad.x+2*(unit+6)+unit/2,pad.y+3*(unit+6)+unit/2]};
};
exports.setup=(w,h,saved=false)=>{
 const width=Math.min(w-32,780),x=(w-width)/2,side=w>=480&&h<500,gap=side?6:10,gallery=side?width-218:width,cols=side?2:w>=680?3:2,rows=6/cols,top=114;
 const ch=Math.max(44,Math.min(side?100:156,(h-top-(side?10:166)-gap*rows)/rows)),cw=(gallery-gap*(cols-1))/cols,sx=side?x+gallery+18:x,sw=side?200:width,sy=side?top:top+rows*(ch+gap)+6,nw=saved?(sw-10)/2:sw;
 return {page:i=>[x+(i+.5)*width/3-2,86],marking:i=>[sx+(i+.5)*(sw+10)/2-5,sy+22],variant:i=>[x+(i%cols)*(cw+gap)+cw/2,top+Math.floor(i/cols)*(ch+gap)+ch/2],difficulty:i=>[sx+(i+.5)*sw/3-2,sy+71],new:[sx+nw/2,sy+129],resume:[sx+nw+10+nw/2,sy+129]};
};
