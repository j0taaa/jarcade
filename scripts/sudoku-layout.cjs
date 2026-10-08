exports.play=(w,h)=>{
  const side=w>=850||w>=480&&h<550;
  let board,pad,actions;
  if(side){const pw=h<450?194:256,size=Math.max(96,Math.min(h-112,w-pw-64,630)),x=(w-size-pw-28)/2,top=h<450?64:100;board={x,y:top,w:size};pad={x:x+size+28,y:top,w:pw};actions=h<450?{x:(w-340)/2,y:h-48,w:340,h:44}:{x:pad.x-4,y:top+pw+12,w:pw+8,h:44};}
  else{const pw=Math.min(w-32,h<700?194:288),size=Math.max(96,Math.min(w-24,h-72-pw-98,600));board={x:(w-size)/2,y:72,w:size};pad={x:(w-pw)/2,y:72+size+16,w:pw};const aw=Math.min(w-24,340);actions={x:(w-aw)/2,y:pad.y+pw+10,w:aw,h:44};}
  const unit=(pad.w-18)/4;
  return {cell:i=>[board.x+(i%9+.5)*board.w/9,board.y+(Math.floor(i/9)+.5)*board.w/9],number:d=>[pad.x+((d-1)%3)*(unit+6)+unit/2,pad.y+Math.floor((d-1)/3)*(unit+6)+unit/2],tool:i=>[pad.x+3*(unit+6)+unit/2,pad.y+i*(unit+6)+unit/2],action:i=>[actions.x+(i+.5)*actions.w/6,actions.y+actions.h/2],erase:[pad.x+unit/2,pad.y+3*(unit+6)+unit/2],candidates:[pad.x+2*(unit+6)+unit/2,pad.y+3*(unit+6)+unit/2]};
};
exports.setup=(w,h,saved=false)=>{
  const width=Math.min(w-32,780),x=(w-width)/2,compact=h<500,cols=w>=680||compact?3:2,rows=6/cols,gap=10,cw=(width-gap*(cols-1))/cols,top=compact?64:80;
  const ch=Math.max(compact?56:68,Math.min(compact?100:156,(h-top-166-gap*(rows-1))/rows));const y=top+rows*(ch+gap)+6;const nw=saved?(width-10)/2:width;
  return {variant:i=>[x+(i%cols+.5)*(cw+gap)-gap/2,top+(Math.floor(i/cols)+.5)*(ch+gap)-gap/2],difficulty:i=>[x+(i+.5)*width/3-2,y+21],new:[x+nw/2,y+42+14+23],resume:[x+nw+10+nw/2,y+42+14+23]};
};
