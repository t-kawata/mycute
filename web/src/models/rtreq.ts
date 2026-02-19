import { UsrType } from 'src/enums/usrtype'

export interface CreateUsrReq {
    name: string
    email: string
    password: string
    bgn_at: string
    end_at: string
    // serde(rename = "type")
    type?: UsrType
    base_point?: number
    belong_rate?: number
    max_works?: number
    flush_days?: number
    rate?: number
    flush_fee_rate?: number
}
