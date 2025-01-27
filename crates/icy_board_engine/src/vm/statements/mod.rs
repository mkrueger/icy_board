#[allow(unused_variables)]
#[allow(clippy::missing_errors_doc)]
#[allow(clippy::missing_panics_doc)]
pub mod predefined_procedures;

use async_recursion::async_recursion;
pub use predefined_procedures::*;

use crate::{
    executable::{OpCode, PPEExpr},
    Res,
};

use super::VirtualMachine;

#[async_recursion(?Send)]
pub async fn run_predefined_statement(opcode: OpCode, arg: &mut VirtualMachine<'_>, arguments: &[PPEExpr]) -> Res<()> {
    match opcode {
        OpCode::END => predefined_procedures::end(arg, arguments).await,
        OpCode::CLS => predefined_procedures::cls(arg, arguments).await,
        OpCode::CLREOL => predefined_procedures::clreol(arg, arguments).await,
        OpCode::MORE => predefined_procedures::more(arg, arguments).await,
        OpCode::WAIT => predefined_procedures::wait(arg, arguments).await,
        OpCode::COLOR => predefined_procedures::color(arg, arguments).await,
        OpCode::GOTO => predefined_procedures::invalid(arg, arguments).await,
        OpCode::LET => predefined_procedures::invalid(arg, arguments).await,
        OpCode::PRINT => predefined_procedures::print(arg, arguments).await,
        OpCode::PRINTLN => predefined_procedures::println(arg, arguments).await,
        OpCode::IFNOT => predefined_procedures::invalid(arg, arguments).await,
        OpCode::CONFFLAG => predefined_procedures::confflag(arg, arguments).await,
        OpCode::CONFUNFLAG => predefined_procedures::confunflag(arg, arguments).await,
        OpCode::DISPFILE => predefined_procedures::dispfile(arg, arguments).await,
        OpCode::INPUT => predefined_procedures::input(arg, arguments).await,
        OpCode::FCREATE => predefined_procedures::fcreate(arg, arguments).await,
        OpCode::FOPEN => predefined_procedures::fopen(arg, arguments).await,
        OpCode::FAPPEND => predefined_procedures::fappend(arg, arguments).await,
        OpCode::FCLOSE => predefined_procedures::fclose(arg, arguments).await,
        OpCode::FGET => predefined_procedures::fget(arg, arguments).await,
        OpCode::FPUT => predefined_procedures::fput(arg, arguments).await,
        OpCode::FPUTLN => predefined_procedures::fputln(arg, arguments).await,
        OpCode::RESETDISP => predefined_procedures::resetdisp(arg, arguments).await,
        OpCode::STARTDISP => predefined_procedures::startdisp(arg, arguments).await,
        OpCode::FPUTPAD => predefined_procedures::fputpad(arg, arguments).await,
        OpCode::HANGUP => predefined_procedures::hangup(arg, arguments).await,
        OpCode::GETUSER => predefined_procedures::getuser(arg, arguments).await,
        OpCode::PUTUSER => predefined_procedures::putuser(arg, arguments).await,
        OpCode::DEFCOLOR => predefined_procedures::defcolor(arg, arguments).await,
        OpCode::DELETE => predefined_procedures::delete(arg, arguments).await,
        OpCode::DELUSER => predefined_procedures::deluser(arg, arguments).await,
        OpCode::ADJTIME => predefined_procedures::adjtime(arg, arguments).await,
        OpCode::LOG => predefined_procedures::log(arg, arguments).await,
        OpCode::INPUTSTR => predefined_procedures::inputstr(arg, arguments).await,
        OpCode::INPUTYN => predefined_procedures::inputyn(arg, arguments).await,
        OpCode::INPUTMONEY => predefined_procedures::inputmoney(arg, arguments).await,
        OpCode::INPUTINT => predefined_procedures::inputint(arg, arguments).await,
        OpCode::INPUTCC => predefined_procedures::inputcc(arg, arguments).await,
        OpCode::INPUTDATE => predefined_procedures::inputdate(arg, arguments).await,
        OpCode::INPUTTIME => predefined_procedures::inputtime(arg, arguments).await,
        OpCode::GOSUB => predefined_procedures::invalid(arg, arguments).await,
        OpCode::RETURN => predefined_procedures::invalid(arg, arguments).await,
        OpCode::PROMPTSTR => predefined_procedures::promptstr(arg, arguments).await,
        OpCode::DTRON => predefined_procedures::dtron(arg, arguments).await,
        OpCode::DTROFF => predefined_procedures::dtroff(arg, arguments).await,
        OpCode::CDCHKON => predefined_procedures::cdchkon(arg, arguments).await,
        OpCode::CDCHKOFF => predefined_procedures::cdchkoff(arg, arguments).await,
        OpCode::DELAY => predefined_procedures::delay(arg, arguments).await,
        OpCode::SENDMODEM => predefined_procedures::sendmodem(arg, arguments).await,
        OpCode::INC => predefined_procedures::inc(arg, arguments).await,
        OpCode::DEC => predefined_procedures::dec(arg, arguments).await,
        OpCode::NEWLINE => predefined_procedures::newline(arg, arguments).await,
        OpCode::NEWLINES => predefined_procedures::newlines(arg, arguments).await,
        OpCode::TOKENIZE => predefined_procedures::tokenize(arg, arguments).await,
        OpCode::GETTOKEN => predefined_procedures::gettoken(arg, arguments).await,
        OpCode::SHELL => predefined_procedures::shell(arg, arguments).await,
        OpCode::DISPTEXT => predefined_procedures::disptext(arg, arguments).await,
        OpCode::STOP => predefined_procedures::stop(arg, arguments).await,
        OpCode::INPUTTEXT => predefined_procedures::inputtext(arg, arguments).await,
        OpCode::BEEP => predefined_procedures::beep(arg, arguments).await,
        OpCode::PUSH => predefined_procedures::push(arg, arguments).await,
        OpCode::POP => predefined_procedures::pop(arg, arguments).await,
        OpCode::KBDSTUFF => predefined_procedures::kbdstuff(arg, arguments).await,
        OpCode::CALL => predefined_procedures::call(arg, arguments).await,
        OpCode::JOIN => predefined_procedures::join(arg, arguments).await,
        OpCode::QUEST => predefined_procedures::quest(arg, arguments).await,
        OpCode::BLT => predefined_procedures::blt(arg, arguments).await,
        OpCode::DIR => predefined_procedures::dir(arg, arguments).await,
        OpCode::KBDFILE => predefined_procedures::kbdfile(arg, arguments).await,
        OpCode::BYE => predefined_procedures::bye(arg, arguments).await,
        OpCode::GOODBYE => predefined_procedures::goodbye(arg, arguments).await,
        OpCode::BROADCAST => predefined_procedures::broadcast(arg, arguments).await,
        OpCode::WAITFOR => predefined_procedures::waitfor(arg, arguments).await,
        OpCode::KBDCHKON => predefined_procedures::kbdchkon(arg, arguments).await,
        OpCode::KBDCHKOFF => predefined_procedures::kbdchkoff(arg, arguments).await,
        OpCode::OPTEXT => predefined_procedures::optext(arg, arguments).await,
        OpCode::DISPSTR => predefined_procedures::dispstr(arg, arguments).await,
        OpCode::RDUNET => predefined_procedures::rdunet(arg, arguments).await,
        OpCode::WRUNET => predefined_procedures::wrunet(arg, arguments).await,
        OpCode::DOINTR => predefined_procedures::dointr(arg, arguments).await,
        OpCode::VARSEG => predefined_procedures::varseg(arg, arguments).await,
        OpCode::VAROFF => predefined_procedures::varoff(arg, arguments).await,
        OpCode::POKEB => predefined_procedures::pokeb(arg, arguments).await,
        OpCode::POKEW => predefined_procedures::pokew(arg, arguments).await,
        OpCode::VARADDR => predefined_procedures::varaddr(arg, arguments).await,
        OpCode::ANSIPOS => predefined_procedures::ansipos(arg, arguments).await,
        OpCode::BACKUP => predefined_procedures::backup(arg, arguments).await,
        OpCode::FORWARD => predefined_procedures::forward(arg, arguments).await,
        OpCode::FRESHLINE => predefined_procedures::freshline(arg, arguments).await,
        OpCode::WRUSYS => predefined_procedures::wrusys(arg, arguments).await,
        OpCode::RDUSYS => predefined_procedures::rdusys(arg, arguments).await,
        OpCode::NEWPWD => predefined_procedures::newpwd(arg, arguments).await,
        OpCode::OPENCAP => predefined_procedures::opencap(arg, arguments).await,
        OpCode::CLOSECAP => predefined_procedures::closecap(arg, arguments).await,
        OpCode::MESSAGE => predefined_procedures::message(arg, arguments).await,
        OpCode::SAVESCRN => predefined_procedures::savescrn(arg, arguments).await,
        OpCode::RESTSCRN => predefined_procedures::restscrn(arg, arguments).await,
        OpCode::SOUND => predefined_procedures::sound(arg, arguments).await,
        OpCode::CHAT => predefined_procedures::chat(arg, arguments).await,
        OpCode::SPRINT => predefined_procedures::sprint(arg, arguments).await,
        OpCode::SPRINTLN => predefined_procedures::sprintln(arg, arguments).await,
        OpCode::MPRINT => predefined_procedures::mprint(arg, arguments).await,
        OpCode::MPRINTLN => predefined_procedures::mprintln(arg, arguments).await,
        OpCode::RENAME => predefined_procedures::rename(arg, arguments).await,
        OpCode::FREWIND => predefined_procedures::frewind(arg, arguments).await,
        OpCode::POKEDW => predefined_procedures::pokedw(arg, arguments).await,
        OpCode::DBGLEVEL => predefined_procedures::dbglevel(arg, arguments).await,
        OpCode::SHOWON => predefined_procedures::showon(arg, arguments).await,
        OpCode::SHOWOFF => predefined_procedures::showoff(arg, arguments).await,
        OpCode::PAGEON => predefined_procedures::pageon(arg, arguments).await,
        OpCode::PAGEOFF => predefined_procedures::pageoff(arg, arguments).await,
        OpCode::FSEEK => predefined_procedures::fseek(arg, arguments).await,
        OpCode::FFLUSH => predefined_procedures::fflush(arg, arguments).await,
        OpCode::FREAD => predefined_procedures::fread(arg, arguments).await,
        OpCode::FWRITE => predefined_procedures::fwrite(arg, arguments).await,
        OpCode::FDEFIN => predefined_procedures::fdefin(arg, arguments).await,
        OpCode::FDEFOUT => predefined_procedures::fdefout(arg, arguments).await,
        OpCode::FDGET => predefined_procedures::fdget(arg, arguments).await,
        OpCode::FDPUT => predefined_procedures::fdput(arg, arguments).await,
        OpCode::FDPUTLN => predefined_procedures::fdputln(arg, arguments).await,
        OpCode::FDPUTPAD => predefined_procedures::fdputpad(arg, arguments).await,
        OpCode::FDREAD => predefined_procedures::fdread(arg, arguments).await,
        OpCode::FDWRITE => predefined_procedures::fdwrite(arg, arguments).await,
        OpCode::ADJBYTES => predefined_procedures::adjbytes(arg, arguments).await,
        OpCode::KBDSTRING => predefined_procedures::kbdstring(arg, arguments).await,
        OpCode::ALIAS => predefined_procedures::alias(arg, arguments).await,
        OpCode::REDIM => predefined_procedures::redim(arg, arguments).await,
        OpCode::APPEND => predefined_procedures::append(arg, arguments).await,
        OpCode::COPY => predefined_procedures::copy(arg, arguments).await,
        OpCode::KBDFLUSH => predefined_procedures::kbdflush(arg, arguments).await,
        OpCode::MDMFLUSH => predefined_procedures::mdmflush(arg, arguments).await,
        OpCode::KEYFLUSH => predefined_procedures::keyflush(arg, arguments).await,
        OpCode::LASTIN => predefined_procedures::lastin(arg, arguments).await,
        OpCode::FLAG => predefined_procedures::flag(arg, arguments).await,
        OpCode::DOWNLOAD => predefined_procedures::download(arg, arguments).await,
        OpCode::WRUSYSDOOR => predefined_procedures::wrusysdoor(arg, arguments).await,
        OpCode::GETALTUSER => predefined_procedures::getaltuser(arg, arguments).await,
        OpCode::ADJDBYTES => predefined_procedures::adjdbytes(arg, arguments).await,
        OpCode::ADJTBYTES => predefined_procedures::adjtbytes(arg, arguments).await,
        OpCode::ADJTFILES => predefined_procedures::ayjtfiles(arg, arguments).await,
        OpCode::LANG => predefined_procedures::lang(arg, arguments).await,
        OpCode::SORT => predefined_procedures::sort(arg, arguments).await,
        OpCode::MOUSEREG => predefined_procedures::mousereg(arg, arguments).await,
        OpCode::SCRFILE => predefined_procedures::scrfile(arg, arguments).await,
        OpCode::SEARCHINIT => predefined_procedures::searchinit(arg, arguments).await,
        OpCode::SEARCHFIND => predefined_procedures::searchfind(arg, arguments).await,
        OpCode::SEARCHSTOP => predefined_procedures::searchstop(arg, arguments).await,
        OpCode::PRFOUND => predefined_procedures::prfound(arg, arguments).await,
        OpCode::PRFOUNDLN => predefined_procedures::prfoundln(arg, arguments).await,
        OpCode::TPAGET => predefined_procedures::tpaget(arg, arguments).await,
        OpCode::TPAPUT => predefined_procedures::tpaput(arg, arguments).await,
        OpCode::TPACGET => predefined_procedures::tpacgea(arg, arguments).await,
        OpCode::TPACPUT => predefined_procedures::tpacput(arg, arguments).await,
        OpCode::TPAREAD => predefined_procedures::tparead(arg, arguments).await,
        OpCode::TPAWRITE => predefined_procedures::tpawrite(arg, arguments).await,
        OpCode::TPACREAD => predefined_procedures::tpacread(arg, arguments).await,
        OpCode::TPACWRITE => predefined_procedures::tpacwrite(arg, arguments).await,
        OpCode::BITSET => predefined_procedures::bitset(arg, arguments).await,
        OpCode::BITCLEAR => predefined_procedures::bitclear(arg, arguments).await,
        OpCode::BRAG => predefined_procedures::brag(arg, arguments).await,
        OpCode::FREALTUSER => predefined_procedures::frealtuser(arg, arguments).await,
        OpCode::SETLMR => predefined_procedures::setlmr(arg, arguments).await,
        OpCode::SETENV => predefined_procedures::setenv(arg, arguments).await,
        OpCode::FCLOSEALL => predefined_procedures::fcloseall(arg, arguments).await,
        OpCode::DECLARE => predefined_procedures::invalid(arg, arguments).await,
        OpCode::FUNCTION => predefined_procedures::invalid(arg, arguments).await,
        OpCode::PROCEDURE => predefined_procedures::invalid(arg, arguments).await,
        OpCode::PCALL => predefined_procedures::invalid(arg, arguments).await,
        OpCode::FPCLR => predefined_procedures::invalid(arg, arguments).await,
        OpCode::BEGIN => predefined_procedures::invalid(arg, arguments).await,
        OpCode::FEND => predefined_procedures::invalid(arg, arguments).await,
        OpCode::STATIC => predefined_procedures::invalid(arg, arguments).await,
        OpCode::STACKABORT => predefined_procedures::stackabort(arg, arguments).await,
        OpCode::DCREATE => predefined_procedures::dcreate(arg, arguments).await,
        OpCode::DOPEN => predefined_procedures::dopen(arg, arguments).await,
        OpCode::DCLOSE => predefined_procedures::dclose(arg, arguments).await,
        OpCode::DSETALIAS => predefined_procedures::dsetalias(arg, arguments).await,
        OpCode::DPACK => predefined_procedures::dpack(arg, arguments).await,
        OpCode::DCLOSEALL => predefined_procedures::dcloseall(arg, arguments).await,
        OpCode::DLOCK => predefined_procedures::dlock(arg, arguments).await,
        OpCode::DLOCKR => predefined_procedures::dlockr(arg, arguments).await,
        OpCode::DLOCKG => predefined_procedures::dlockg(arg, arguments).await,
        OpCode::DUNLOCK => predefined_procedures::dunlock(arg, arguments).await,
        OpCode::DNCREATE => predefined_procedures::dncreate(arg, arguments).await,
        OpCode::DNOPEN => predefined_procedures::dnopen(arg, arguments).await,
        OpCode::DNCLOSE => predefined_procedures::dnclose(arg, arguments).await,
        OpCode::DNCLOSEALL => predefined_procedures::dncloseall(arg, arguments).await,
        OpCode::DNEW => predefined_procedures::dnew(arg, arguments).await,
        OpCode::DADD => predefined_procedures::dadd(arg, arguments).await,
        OpCode::DAPPEND => predefined_procedures::dappend(arg, arguments).await,
        OpCode::DTOP => predefined_procedures::dtop(arg, arguments).await,
        OpCode::DGO => predefined_procedures::dgo(arg, arguments).await,
        OpCode::DBOTTOM => predefined_procedures::dbottom(arg, arguments).await,
        OpCode::DSKIP => predefined_procedures::dskip(arg, arguments).await,
        OpCode::DBLANK => predefined_procedures::dblank(arg, arguments).await,
        OpCode::DDELETE => predefined_procedures::ddelete(arg, arguments).await,
        OpCode::DRECALL => predefined_procedures::drecall(arg, arguments).await,
        OpCode::DTAG => predefined_procedures::dtag(arg, arguments).await,
        OpCode::DSEEK => predefined_procedures::dseek(arg, arguments).await,
        OpCode::DFBLANK => predefined_procedures::dfblank(arg, arguments).await,
        OpCode::DGET => predefined_procedures::dget(arg, arguments).await,
        OpCode::DPUT => predefined_procedures::dput(arg, arguments).await,
        OpCode::DFCOPY => predefined_procedures::dfcopy(arg, arguments).await,
        OpCode::EVAL => { predefined_procedures::eval(arg, arguments).await },
        OpCode::ACCOUNT => predefined_procedures::account(arg, arguments).await,
        OpCode::RECORDUSAGE => predefined_procedures::recordusage(arg, arguments).await,
        OpCode::MSGTOFILE => predefined_procedures::msgtofile(arg, arguments).await,
        OpCode::QWKLIMITS => predefined_procedures::qwklimits(arg, arguments).await,
        OpCode::COMMAND => predefined_procedures::command(arg, arguments).await,
        OpCode::USELMRS => predefined_procedures::uselmrs(arg, arguments).await,
        OpCode::CONFINFO => predefined_procedures::confinfo(arg, arguments).await,
        OpCode::ADJTUBYTES => predefined_procedures::adjtubytes(arg, arguments).await,
        OpCode::GRAFMODE => predefined_procedures::grafmode(arg, arguments).await,
        OpCode::ADDUSER => predefined_procedures::adduser(arg, arguments).await,
        OpCode::KILLMSG => predefined_procedures::killmsg(arg, arguments).await,
        OpCode::CHDIR => predefined_procedures::chdir(arg, arguments).await,
        OpCode::MKDIR => predefined_procedures::mkdir(arg, arguments).await,
        OpCode::RMDIR => predefined_procedures::rmdir(arg, arguments).await,
        OpCode::FDOWRAKA => predefined_procedures::fdowraka(arg, arguments).await,
        OpCode::FDOADDAKA => predefined_procedures::fdoaddaka(arg, arguments).await,
        OpCode::FDOWRORG => predefined_procedures::fdowrorg(arg, arguments).await,
        OpCode::FDOADDORG => predefined_procedures::fdoaddorg(arg, arguments).await,
        OpCode::FDOQMOD => predefined_procedures::fdoqmod(arg, arguments).await,
        OpCode::FDOQADD => predefined_procedures::fdoqadd(arg, arguments).await,
        OpCode::FDOQDEL => predefined_procedures::fdoqdel(arg, arguments).await,
        OpCode::SOUNDDELAY => predefined_procedures::sounddelay(arg, arguments).await,

        OpCode::ShortDesc => predefined_procedures::invalid(arg, arguments).await,
        OpCode::MoveMsg => predefined_procedures::invalid(arg, arguments).await,
        OpCode::SetBankBal => predefined_procedures::invalid(arg, arguments).await,
    }
}
