let SessionLoad = 1
let s:so_save = &so | let s:siso_save = &siso | set so=0 siso=0
let v:this_session=expand("<sfile>:p")
silent only
cd ~/rustos
if expand('%') == '' && !&modified && line('$') <= 1 && getline(1) == ''
  let s:wipebuf = bufnr('%')
endif
set shortmess=aoO
badd +83 kern/src/allocator.rs
badd +165 kern/src/allocator/tests.rs
badd +1 lib/pi/src/atags
badd +77 lib/pi/src/atags/atag.rs
badd +69 lib/pi/src/atags/raw.rs
badd +1 ~/rustos
badd +1060 term://.//129997:/usr/bin/fish
badd +1 kern/src/allocator/bump.rs
badd +1 kern/src/allocator/util.rs
badd +866 term://.//134850:/usr/bin/fish
badd +385 term://.//136027:/usr/bin/fish
badd +819 term://.//137004:/usr/bin/fish
badd +1933 term://.//138843:/usr/bin/fish
badd +70 kern/src/allocator/bin.rs
badd +1 kern/src/allocator/linked_list.rs
argglobal
%argdel
$argadd ~/rustos
edit kern/src/allocator/linked_list.rs
set splitbelow splitright
wincmd t
set winminheight=0
set winheight=1
set winminwidth=0
set winwidth=1
argglobal
setlocal fdm=manual
setlocal fde=0
setlocal fmr={{{,}}}
setlocal fdi=#
setlocal fdl=0
setlocal fml=1
setlocal fdn=20
setlocal fen
silent! normal! zE
let s:l = 83 - ((16 * winheight(0) + 44) / 89)
if s:l < 1 | let s:l = 1 | endif
exe s:l
normal! zt
83
normal! 05|
tabnext 1
if exists('s:wipebuf') && getbufvar(s:wipebuf, '&buftype') isnot# 'terminal'
  silent exe 'bwipe ' . s:wipebuf
endif
unlet! s:wipebuf
set winheight=1 winwidth=20 winminheight=1 winminwidth=1 shortmess=filnxtToOFcI
let s:sx = expand("<sfile>:p:r")."x.vim"
if file_readable(s:sx)
  exe "source " . fnameescape(s:sx)
endif
let &so = s:so_save | let &siso = s:siso_save
let g:this_session = v:this_session
let g:this_obsession = v:this_session
doautoall SessionLoadPost
unlet SessionLoad
" vim: set ft=vim :
