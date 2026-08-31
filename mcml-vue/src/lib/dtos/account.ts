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