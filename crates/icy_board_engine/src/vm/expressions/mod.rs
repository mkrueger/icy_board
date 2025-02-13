#[allow(unused_variables)]
#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub mod predefined_functions;

pub use predefined_functions::*;

use crate::{
    executable::{FuncOpCode, PPEExpr, VariableValue},
    Res,
};

use super::VirtualMachine;

pub async fn run_function(opcode: FuncOpCode, arg: &mut VirtualMachine<'_>, arguments: &[PPEExpr]) -> Res<VariableValue> {
    match opcode {
        FuncOpCode::END => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::CPAR => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::UPLUS => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::UMINUS => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::EXP => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::TIMES => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::DIVIDE => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::MOD => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::PLUS => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::MINUS => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::EQ => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::NE => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::LT => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::LE => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::GT => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::GE => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::NOT => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::AND => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::OR => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::OPAR => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::LEN => predefined_functions::len(arg, arguments).await,
        FuncOpCode::LOWER => predefined_functions::lower(arg, arguments).await,
        FuncOpCode::UPPER => predefined_functions::upper(arg, arguments).await,
        FuncOpCode::MID => predefined_functions::mid(arg, arguments).await,
        FuncOpCode::LEFT => predefined_functions::left(arg, arguments).await,
        FuncOpCode::RIGHT => predefined_functions::right(arg, arguments).await,
        FuncOpCode::SPACE => predefined_functions::space(arg, arguments).await,
        FuncOpCode::FERR => predefined_functions::ferr(arg, arguments).await,
        FuncOpCode::CHR => predefined_functions::chr(arg, arguments).await,
        FuncOpCode::ASC => predefined_functions::asc(arg, arguments).await,
        FuncOpCode::INSTR => predefined_functions::instr(arg, arguments).await,
        FuncOpCode::ABORT => predefined_functions::abort(arg, arguments).await,
        FuncOpCode::LTRIM => predefined_functions::ltrim(arg, arguments).await,
        FuncOpCode::RTRIM => predefined_functions::rtrim(arg, arguments).await,
        FuncOpCode::TRIM => predefined_functions::trim(arg, arguments).await,
        FuncOpCode::RANDOM => predefined_functions::random(arg, arguments).await,
        FuncOpCode::DATE => predefined_functions::date(arg, arguments).await,
        FuncOpCode::TIME => predefined_functions::time(arg, arguments).await,
        FuncOpCode::U_NAME => predefined_functions::u_name(arg, arguments).await,
        FuncOpCode::U_LDATE => predefined_functions::u_ldate(arg, arguments).await,
        FuncOpCode::U_LTIME => predefined_functions::u_ltime(arg, arguments).await,
        FuncOpCode::U_LDIR => predefined_functions::u_ldir(arg, arguments).await,
        FuncOpCode::U_LOGONS => predefined_functions::u_logons(arg, arguments).await,
        FuncOpCode::U_FUL => predefined_functions::u_ful(arg, arguments).await,
        FuncOpCode::U_FDL => predefined_functions::u_fdl(arg, arguments).await,
        FuncOpCode::U_BDLDAY => predefined_functions::u_bdlday(arg, arguments).await,
        FuncOpCode::U_TIMEON => predefined_functions::u_timeon(arg, arguments).await,
        FuncOpCode::U_BDL => predefined_functions::u_bdl(arg, arguments).await,
        FuncOpCode::U_BUL => predefined_functions::u_bul(arg, arguments).await,
        FuncOpCode::YEAR => predefined_functions::year(arg, arguments).await,
        FuncOpCode::MONTH => predefined_functions::month(arg, arguments).await,
        FuncOpCode::DAY => predefined_functions::day(arg, arguments).await,
        FuncOpCode::DOW => predefined_functions::dow(arg, arguments).await,
        FuncOpCode::HOUR => predefined_functions::hour(arg, arguments).await,
        FuncOpCode::MIN => predefined_functions::min(arg, arguments).await,
        FuncOpCode::SEC => predefined_functions::sec(arg, arguments).await,
        FuncOpCode::TIMEAP => predefined_functions::timeap(arg, arguments).await,
        FuncOpCode::VER => predefined_functions::ver(arg, arguments).await,
        FuncOpCode::NOCHAR => predefined_functions::nochar(arg, arguments).await,
        FuncOpCode::YESCHAR => predefined_functions::yeschar(arg, arguments).await,
        FuncOpCode::STRIPATX => predefined_functions::stripatx(arg, arguments).await,
        FuncOpCode::REPLACE => predefined_functions::replace(arg, arguments).await,
        FuncOpCode::STRIP => predefined_functions::strip(arg, arguments).await,
        FuncOpCode::INKEY => predefined_functions::inkey(arg, arguments).await,
        FuncOpCode::TOSTRING => predefined_functions::tostring(arg, arguments).await,
        FuncOpCode::MASK_PWD => predefined_functions::mask_pwd(arg, arguments).await,
        FuncOpCode::MASK_ALPHA => predefined_functions::mask_alpha(arg, arguments).await,
        FuncOpCode::MASK_NUM => predefined_functions::mask_num(arg, arguments).await,
        FuncOpCode::MASK_ALNUM => predefined_functions::mask_alnum(arg, arguments).await,
        FuncOpCode::MASK_FILE => predefined_functions::mask_file(arg, arguments).await,
        FuncOpCode::MASK_PATH => predefined_functions::mask_path(arg, arguments).await,
        FuncOpCode::MASK_ASCII => predefined_functions::mask_ascii(arg, arguments).await,
        FuncOpCode::CURCONF => predefined_functions::curconf(arg, arguments).await,
        FuncOpCode::PCBDAT => predefined_functions::pcbdat(arg, arguments).await,
        FuncOpCode::PPEPATH => predefined_functions::ppepath(arg, arguments).await,
        FuncOpCode::VALDATE => predefined_functions::valdate(arg, arguments).await,
        FuncOpCode::VALTIME => predefined_functions::valtime(arg, arguments).await,
        FuncOpCode::U_MSGRD => predefined_functions::u_msgrd(arg, arguments).await,
        FuncOpCode::U_MSGWR => predefined_functions::u_msgwr(arg, arguments).await,
        FuncOpCode::PCBNODE => predefined_functions::pcbnode(arg, arguments).await,
        FuncOpCode::READLINE => predefined_functions::readline(arg, arguments).await,
        FuncOpCode::SYSOPSEC => predefined_functions::sysopsec(arg, arguments).await,
        FuncOpCode::ONLOCAL => predefined_functions::onlocal(arg, arguments).await,
        FuncOpCode::UN_STAT => predefined_functions::un_stat(arg, arguments).await,
        FuncOpCode::UN_NAME => predefined_functions::un_name(arg, arguments).await,
        FuncOpCode::UN_CITY => predefined_functions::un_city(arg, arguments).await,
        FuncOpCode::UN_OPER => predefined_functions::un_oper(arg, arguments).await,
        FuncOpCode::CURSEC => predefined_functions::cursec(arg, arguments).await,
        FuncOpCode::GETTOKEN => predefined_functions::gettoken(arg, arguments).await,
        FuncOpCode::MINLEFT => predefined_functions::minleft(arg, arguments).await,
        FuncOpCode::MINON => predefined_functions::minon(arg, arguments).await,
        FuncOpCode::GETENV => predefined_functions::getenv(arg, arguments).await,
        FuncOpCode::CALLID => predefined_functions::callid(arg, arguments).await,
        FuncOpCode::REGAL => predefined_functions::regal(arg, arguments).await,
        FuncOpCode::REGAH => predefined_functions::regah(arg, arguments).await,
        FuncOpCode::REGBL => predefined_functions::regbl(arg, arguments).await,
        FuncOpCode::REGBH => predefined_functions::regbh(arg, arguments).await,
        FuncOpCode::REGCL => predefined_functions::regcl(arg, arguments).await,
        FuncOpCode::REGCH => predefined_functions::regch(arg, arguments).await,
        FuncOpCode::REGDL => predefined_functions::regdl(arg, arguments).await,
        FuncOpCode::REGDH => predefined_functions::regdh(arg, arguments).await,
        FuncOpCode::REGAX => predefined_functions::regax(arg, arguments).await,
        FuncOpCode::REGBX => predefined_functions::regbx(arg, arguments).await,
        FuncOpCode::REGCX => predefined_functions::regcx(arg, arguments).await,
        FuncOpCode::REGDX => predefined_functions::regdx(arg, arguments).await,
        FuncOpCode::REGSI => predefined_functions::regsi(arg, arguments).await,
        FuncOpCode::REGDI => predefined_functions::regdi(arg, arguments).await,
        FuncOpCode::REGF => predefined_functions::regf(arg, arguments).await,
        FuncOpCode::REGCF => predefined_functions::regcf(arg, arguments).await,
        FuncOpCode::REGDS => predefined_functions::regds(arg, arguments).await,
        FuncOpCode::REGES => predefined_functions::reges(arg, arguments).await,
        FuncOpCode::B2W => predefined_functions::b2w(arg, arguments).await,
        FuncOpCode::PEEKB => predefined_functions::peekb(arg, arguments).await,
        FuncOpCode::PEEKW => predefined_functions::peekw(arg, arguments).await,
        FuncOpCode::MKADDR => predefined_functions::mkaddr(arg, arguments).await,
        FuncOpCode::EXIST => predefined_functions::exist(arg, arguments).await,
        FuncOpCode::I2S => predefined_functions::i2s(arg, arguments).await,
        FuncOpCode::S2I => predefined_functions::s2i(arg, arguments).await,
        FuncOpCode::CARRIER => predefined_functions::carrier(arg, arguments).await,
        FuncOpCode::TOKENSTR => predefined_functions::tokenstr(arg, arguments).await,
        FuncOpCode::CDON => predefined_functions::cdon(arg, arguments).await,
        FuncOpCode::LANGEXT => predefined_functions::langext(arg, arguments).await,
        FuncOpCode::ANSION => predefined_functions::ansion(arg, arguments).await,
        FuncOpCode::VALCC => predefined_functions::valcc(arg, arguments).await,
        FuncOpCode::FMTCC => predefined_functions::fmtcc(arg, arguments).await,
        FuncOpCode::CCTYPE => predefined_functions::cctype(arg, arguments).await,
        FuncOpCode::GETX => predefined_functions::getx(arg, arguments).await,
        FuncOpCode::GETY => predefined_functions::gety(arg, arguments).await,
        FuncOpCode::BAND => predefined_functions::band(arg, arguments).await,
        FuncOpCode::BOR => predefined_functions::bor(arg, arguments).await,
        FuncOpCode::BXOR => predefined_functions::bxor(arg, arguments).await,
        FuncOpCode::BNOT => predefined_functions::bnot(arg, arguments).await,
        FuncOpCode::U_PWDHIST => predefined_functions::u_pwdhist(arg, arguments).await,
        FuncOpCode::U_PWDLC => predefined_functions::u_pwdlc(arg, arguments).await,
        FuncOpCode::U_PWDTC => predefined_functions::u_pwdtc(arg, arguments).await,
        FuncOpCode::U_STAT => predefined_functions::u_stat(arg, arguments).await,
        FuncOpCode::DEFCOLOR => predefined_functions::defcolor(arg, arguments).await,
        FuncOpCode::ABS => predefined_functions::abs(arg, arguments).await,
        FuncOpCode::GRAFMODE => predefined_functions::grafmode(arg, arguments).await,
        FuncOpCode::PSA => predefined_functions::psa(arg, arguments).await,
        FuncOpCode::FILEINF => predefined_functions::fileinf(arg, arguments).await,
        FuncOpCode::PPENAME => predefined_functions::ppename(arg, arguments).await,
        FuncOpCode::MKDATE => predefined_functions::mkdate(arg, arguments).await,
        FuncOpCode::CURCOLOR => predefined_functions::curcolor(arg, arguments).await,
        FuncOpCode::KINKEY => predefined_functions::kinkey(arg, arguments).await,
        FuncOpCode::MINKEY => predefined_functions::minkey(arg, arguments).await,
        FuncOpCode::MAXNODE => predefined_functions::maxnode(arg, arguments).await,
        FuncOpCode::SLPATH => predefined_functions::slpath(arg, arguments).await,
        FuncOpCode::HELPPATH => predefined_functions::helppath(arg, arguments).await,
        FuncOpCode::TEMPPATH => predefined_functions::temppath(arg, arguments).await,
        FuncOpCode::MODEM => predefined_functions::modem(arg, arguments).await,
        FuncOpCode::LOGGEDON => predefined_functions::loggedon(arg, arguments).await,
        FuncOpCode::CALLNUM => predefined_functions::callnum(arg, arguments).await,
        FuncOpCode::MGETBYTE => predefined_functions::mgetbyte(arg, arguments).await,
        FuncOpCode::TOKCOUNT => predefined_functions::tokcount(arg, arguments).await,
        FuncOpCode::U_RECNUM => predefined_functions::u_recnum(arg, arguments).await,
        FuncOpCode::U_INCONF => predefined_functions::u_inconf(arg, arguments).await,
        FuncOpCode::PEEKDW => predefined_functions::peekdw(arg, arguments).await,
        FuncOpCode::DBGLEVEL => predefined_functions::dbglevel(arg, arguments).await,
        FuncOpCode::SCRTEXT => predefined_functions::scrtext(arg, arguments).await,
        FuncOpCode::SHOWSTAT => predefined_functions::showstat(arg, arguments).await,
        FuncOpCode::PAGESTAT => predefined_functions::pagestat(arg, arguments).await,
        FuncOpCode::REPLACESTR => predefined_functions::replacestr(arg, arguments).await,
        FuncOpCode::STRIPSTR => predefined_functions::stripstr(arg, arguments).await,
        FuncOpCode::TOBIGSTR => predefined_functions::tobigstr(arg, arguments).await,
        FuncOpCode::TOBOOLEAN => predefined_functions::toboolean(arg, arguments).await,
        FuncOpCode::TOBYTE => predefined_functions::tobyte(arg, arguments).await,
        FuncOpCode::TODATE => predefined_functions::todate(arg, arguments).await,
        FuncOpCode::TODREAL => predefined_functions::todreal(arg, arguments).await,
        FuncOpCode::TOEDATE => predefined_functions::toedate(arg, arguments).await,
        FuncOpCode::TOINTEGER => predefined_functions::tointeger(arg, arguments).await,
        FuncOpCode::TOMONEY => predefined_functions::tomoney(arg, arguments).await,
        FuncOpCode::TOREAL => predefined_functions::toreal(arg, arguments).await,
        FuncOpCode::TOSBYTE => predefined_functions::tosbyte(arg, arguments).await,
        FuncOpCode::TOSWORD => predefined_functions::tosword(arg, arguments).await,
        FuncOpCode::TOTIME => predefined_functions::totime(arg, arguments).await,
        FuncOpCode::TOUNSIGNED => predefined_functions::tounsigned(arg, arguments).await,
        FuncOpCode::TOWORD => predefined_functions::toword(arg, arguments).await,
        FuncOpCode::MIXED => predefined_functions::mixed(arg, arguments).await,
        FuncOpCode::ALIAS => predefined_functions::alias(arg, arguments).await,
        FuncOpCode::CONFREG => predefined_functions::confreg(arg, arguments).await,
        FuncOpCode::CONFEXP => predefined_functions::confexp(arg, arguments).await,
        FuncOpCode::CONFSEL => predefined_functions::confsel(arg, arguments).await,
        FuncOpCode::CONFSYS => predefined_functions::confsys(arg, arguments).await,
        FuncOpCode::CONFMW => predefined_functions::confmw(arg, arguments).await,
        FuncOpCode::LPRINTED => predefined_functions::lprinted(arg, arguments).await,
        FuncOpCode::ISNONSTOP => predefined_functions::isnonstop(arg, arguments).await,
        FuncOpCode::ERRCORRECT => predefined_functions::errcorrect(arg, arguments).await,
        FuncOpCode::CONFALIAS => predefined_functions::confalias(arg, arguments).await,
        FuncOpCode::USERALIAS => predefined_functions::useralias(arg, arguments).await,
        FuncOpCode::CURUSER => predefined_functions::curuser(arg, arguments).await,
        FuncOpCode::U_LMR => predefined_functions::u_lmr(arg, arguments).await,
        FuncOpCode::CHATSTAT => predefined_functions::chatstat(arg, arguments).await,
        FuncOpCode::DEFANS => predefined_functions::defans(arg, arguments).await,
        FuncOpCode::LASTANS => predefined_functions::lastans(arg, arguments).await,
        FuncOpCode::MEGANUM => predefined_functions::meganum(arg, arguments).await,
        FuncOpCode::EVTTIMEADJ => predefined_functions::evttimeadj(arg, arguments).await,
        FuncOpCode::ISBITSET => predefined_functions::isbitset(arg, arguments).await,
        FuncOpCode::FMTREAL => predefined_functions::fmtreal(arg, arguments).await,
        FuncOpCode::FLAGCNT => predefined_functions::flagcnt(arg, arguments).await,
        FuncOpCode::KBDBUFSIZE => predefined_functions::kbdbufsize(arg, arguments).await,
        FuncOpCode::PPLBUFSIZE => predefined_functions::pplbufsize(arg, arguments).await,
        FuncOpCode::KBDFILUSED => predefined_functions::kbdfilusued(arg, arguments).await,
        FuncOpCode::LOMSGNUM => predefined_functions::lomsgnum(arg, arguments).await,
        FuncOpCode::HIMSGNUM => predefined_functions::himsgnum(arg, arguments).await,
        FuncOpCode::DRIVESPACE => predefined_functions::drivespace(arg, arguments).await,
        FuncOpCode::OUTBYTES => predefined_functions::outbytes(arg, arguments).await,
        FuncOpCode::HICONFNUM => predefined_functions::hiconfnum(arg, arguments).await,
        FuncOpCode::INBYTES => predefined_functions::inbytes(arg, arguments).await,
        FuncOpCode::CRC32 => predefined_functions::crc32(arg, arguments).await,
        FuncOpCode::PCBMAC => predefined_functions::pcbmac(arg, arguments).await,
        FuncOpCode::ACTMSGNUM => predefined_functions::actmsgnum(arg, arguments).await,
        FuncOpCode::STACKLEFT => predefined_functions::stackleft(arg, arguments).await,
        FuncOpCode::STACKERR => predefined_functions::stackerr(arg, arguments).await,
        FuncOpCode::DGETALIAS => predefined_functions::dgetalias(arg, arguments).await,
        FuncOpCode::DBOF => predefined_functions::dbof(arg, arguments).await,
        FuncOpCode::DCHANGED => predefined_functions::dchanged(arg, arguments).await,
        FuncOpCode::DDECIMALS => predefined_functions::ddecimals(arg, arguments).await,
        FuncOpCode::DDELETED => predefined_functions::ddeleted(arg, arguments).await,
        FuncOpCode::DEOF => predefined_functions::deof(arg, arguments).await,
        FuncOpCode::DERR => predefined_functions::derr(arg, arguments).await,
        FuncOpCode::DFIELDS => predefined_functions::dfields(arg, arguments).await,
        FuncOpCode::DLENGTH => predefined_functions::dlength(arg, arguments).await,
        FuncOpCode::DNAME => predefined_functions::dname(arg, arguments).await,
        FuncOpCode::DRECCOUNT => predefined_functions::dreccount(arg, arguments).await,
        FuncOpCode::DRECNO => predefined_functions::drecno(arg, arguments).await,
        FuncOpCode::DTYPE => predefined_functions::dtype(arg, arguments).await,
        FuncOpCode::FNEXT => predefined_functions::fnext(arg, arguments).await,
        FuncOpCode::DNEXT => predefined_functions::dnext(arg, arguments).await,
        FuncOpCode::TODDATE => predefined_functions::toddate(arg, arguments).await,
        FuncOpCode::DCLOSEALL => predefined_functions::dcloseall(arg, arguments).await,
        FuncOpCode::DOPEN => predefined_functions::dopen(arg, arguments).await,
        FuncOpCode::DCLOSE => predefined_functions::dclose(arg, arguments).await,
        FuncOpCode::DSETALIAS => predefined_functions::dsetalias(arg, arguments).await,
        FuncOpCode::DPACK => predefined_functions::dpack(arg, arguments).await,
        FuncOpCode::DLOCKF => predefined_functions::dlockf(arg, arguments).await,
        FuncOpCode::DLOCK => predefined_functions::dlock(arg, arguments).await,
        FuncOpCode::DLOCKR => predefined_functions::dlockr(arg, arguments).await,
        FuncOpCode::DUNLOCK => predefined_functions::dunlock(arg, arguments).await,
        FuncOpCode::DNOPEN => predefined_functions::dnopen(arg, arguments).await,
        FuncOpCode::DNCLOSE => predefined_functions::dnclose(arg, arguments).await,
        FuncOpCode::DNCLOSEALL => predefined_functions::dncloseall(arg, arguments).await,
        FuncOpCode::DNEW => predefined_functions::dnew(arg, arguments).await,
        FuncOpCode::DADD => predefined_functions::dadd(arg, arguments).await,
        FuncOpCode::DAPPEND => predefined_functions::dappend(arg, arguments).await,
        FuncOpCode::DTOP => predefined_functions::dtop(arg, arguments).await,
        FuncOpCode::DGO => predefined_functions::dgo(arg, arguments).await,
        FuncOpCode::DBOTTOM => predefined_functions::dbottom(arg, arguments).await,
        FuncOpCode::DSKIP => predefined_functions::dskip(arg, arguments).await,
        FuncOpCode::DBLANK => predefined_functions::dblank(arg, arguments).await,
        FuncOpCode::DDELETE => predefined_functions::ddelete(arg, arguments).await,
        FuncOpCode::DRECALL => predefined_functions::drecall(arg, arguments).await,
        FuncOpCode::DTAG => predefined_functions::dtag(arg, arguments).await,
        FuncOpCode::DSEEK => predefined_functions::dseek(arg, arguments).await,
        FuncOpCode::DFBLANK => predefined_functions::dfblank(arg, arguments).await,
        FuncOpCode::DGET => predefined_functions::dget(arg, arguments).await,
        FuncOpCode::DPUT => predefined_functions::dput(arg, arguments).await,
        FuncOpCode::DFCOPY => predefined_functions::dfcopy(arg, arguments).await,
        FuncOpCode::DSELECT => predefined_functions::dselect(arg, arguments).await,
        FuncOpCode::DCHKSTAT => predefined_functions::dchkstat(arg, arguments).await,
        FuncOpCode::PCBACCOUNT => predefined_functions::pcbaccount(arg, arguments).await,
        FuncOpCode::PCBACCSTAT => predefined_functions::pcbaccstat(arg, arguments).await,
        FuncOpCode::DERRMSG => predefined_functions::derrmsg(arg, arguments).await,
        FuncOpCode::ACCOUNT => predefined_functions::account(arg, arguments).await,
        FuncOpCode::SCANMSGHDR => predefined_functions::scanmsghdr(arg, arguments).await,
        FuncOpCode::CHECKRIP => predefined_functions::checkrip(arg, arguments).await,
        FuncOpCode::RIPVER => predefined_functions::ripver(arg, arguments).await,
        FuncOpCode::QWKLIMITS => predefined_functions::qwklimits(arg, arguments).await,
        FuncOpCode::FINDFIRST => predefined_functions::findfirst(arg, arguments).await,
        FuncOpCode::FINDNEXT => predefined_functions::findnext(arg, arguments).await,
        FuncOpCode::USELMRS => predefined_functions::uselmrs(arg, arguments).await,
        FuncOpCode::CONFINFO => predefined_functions::confinfo(arg, arguments).await,
        FuncOpCode::TINKEY => predefined_functions::tinkey(arg, arguments).await,
        FuncOpCode::CWD => predefined_functions::cwd(arg, arguments).await,
        FuncOpCode::INSTRR => predefined_functions::instrr(arg, arguments).await,
        FuncOpCode::FDORDAKA => predefined_functions::fdordaka(arg, arguments).await,
        FuncOpCode::FDORDORG => predefined_functions::fdordorg(arg, arguments).await,
        FuncOpCode::FDORDAREA => predefined_functions::fdordarea(arg, arguments).await,
        FuncOpCode::FDOQRD => predefined_functions::fdoqrd(arg, arguments).await,
        FuncOpCode::GETDRIVE => predefined_functions::getdrive(arg, arguments).await,
        FuncOpCode::SETDRIVE => predefined_functions::setdrive(arg, arguments).await,
        FuncOpCode::BS2I => predefined_functions::bs2i(arg, arguments).await,
        FuncOpCode::BD2I => predefined_functions::bd2i(arg, arguments).await,
        FuncOpCode::I2BS => predefined_functions::i2bs(arg, arguments).await,
        FuncOpCode::I2BD => predefined_functions::i2bd(arg, arguments).await,
        FuncOpCode::FTELL => predefined_functions::ftell(arg, arguments).await,
        FuncOpCode::OS => predefined_functions::os(arg, arguments).await,
        FuncOpCode::SHORT_DESC => predefined_functions::shortdesc(arg, arguments).await,
        FuncOpCode::GetBankBal => predefined_functions::getbankbal(arg, arguments).await,
        FuncOpCode::GetMsgHdr => predefined_functions::getmsghdr(arg, arguments).await,
        FuncOpCode::SetMsgHdr => predefined_functions::setmsghdr(arg, arguments).await,
        FuncOpCode::MemberReference => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::MemberCall => predefined_functions::invalid(arg, arguments).await,
        FuncOpCode::NewConfInfo => predefined_functions::new_confinfo(arg, arguments).await,
        FuncOpCode::AreaId => predefined_functions::area_id(arg, arguments).await,
    }
}
