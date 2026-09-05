/**
 * OAuth登录开始
 */
export interface AccountOAuthDto {
  /**
   * 登录码
   */
  code: string;
  /**
   * 登录网址
   */
  url: string;
}

/**
 * OAuth登录阶段进度
 * state：waiting / xbox / xsts / token / profile / ok / fail
 */
export interface AccountOAuthStateDto {
  state: string;
  /** 附加信息（fail 时的错误文案） */
  message: string | null;
}