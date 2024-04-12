use std::io::Write;

pub fn compress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    // compressor's params: buffer size, level, LZ77 window size
    // https://github.com/google/brotli/blob/master/c/tools/brotli.md
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 24);
    writer.write_all(input)?;
    Ok(writer.into_inner())
    // Ok(input.to_vec())
}
//
pub fn decompress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut writer = brotli::DecompressorWriter::new(Vec::new(), 4096);
    writer.write_all(input)?;
    match writer.into_inner() {
        Ok(w) | Err(w) => Ok(w),
    }
    // Ok(input.to_vec())
}

#[test]
fn test_compde() -> anyhow::Result<()> {
    let inp = r#"\CAN/webhook/products/update\EOTPOST\SI\NUL\EOT\n\SO\ACK\SI\f\SI\DLE\DC1\NAK\NAK\NAK\SI\SYN\DC4hostuser-agentcontent-lengthacceptaccept-encodingcontent-typex-forwarded-forx-forwarded-hostx-forwarded-protox-shopify-api-versionx-shopify-hmac-sha256x-shopify-shop-domainx-shopify-topicx-shopify-triggered-atx-shopify-webhook-id\n\255\DEL\NUL \DC4\EOT\ETX'\DLE\SO \ENQ\a,\NAK\v\RS$\NULc80d-14-231-161-9.ngrok-free.appShopify-Captain-Hook1611*/*gzip;q=1.0,deflate;q=0.6,identity;q=0.3application/json35.243.226.186c80d-14-231-161-9.ngrok-free.apphttps2024-01dm86SgC9l21hwNr6Zxfvuayt2j70+a1vapSo70ZF9Vo=feeder8.myshopify.comshop/update2024-04-01T22:30:55.108492337Zd200ce99-6f33-4ac9-a3fb-5018d3fb20f3\NUL\255\EOTK\ACK{\"id\":86498083122,\"name\":\"feeder8\",\"email\":\"nhatanh02@gmail.com\",\"domain\":\"feeder8.myshopify.com\",\"province\":null,\"country\":\"VN\",\"address1\":null,\"zip\":null,\"city\":null,\"source\":null,\"phone\":null,\"latitude\":null,\"longitude\":null,\"primary_locale\":\"en\",\"address2\":null,\"created_at\":\"2024-03-05T14:51:52+07:00\",\"updated_at\":\"2024-04-02T05:30:55+07:00\",\"country_code\":\"VN\",\"country_name\":\"Vietnam\",\"currency\":\"VND\",\"customer_email\":\"nhatanh02@gmail.com\",\"timezone\":\"(GMT+07:00) Asia\\/Jakarta\",\"iana_timezone\":\"Asia\\/Jakarta\",\"shop_owner\":\"Anh Ngo\",\"money_format\":\"{{amount_no_decimals_with_comma_separator}}\226\130\171\",\"money_with_currency_format\":\"{{amount_no_decimals_with_comma_separator}} VND\",\"weight_unit\":\"kg\",\"province_code\":null,\"taxes_included\":false,\"auto_configure_tax_inclusivity\":null,\"tax_shipping\":null,\"county_taxes\":true,\"plan_display_name\":\"Developer Preview\",\"plan_name\":\"partner_test\",\"has_discounts\":true,\"has_gift_cards\":true,\"myshopify_domain\":\"feeder8.myshopify.com\",\"google_apps_domain\":null,\"google_apps_login_enabled\":null,\"money_in_emails_format\":\"{{amount_no_decimals_with_comma_separator}}\226\130\171\",\"money_with_currency_in_emails_format\":\"{{amount_no_decimals_with_comma_separator}} VND\",\"eligible_for_payments\":false,\"requires_extra_payments_agreement\":false,\"password_enabled\":true,\"has_storefront\":true,\"finances\":true,\"primary_location_id\":96514998578,\"checkout_api_supported\":true,\"multi_location_enabled\":true,\"setup_required\":false,\"pre_launch_enabled\":false,\"enabled_presentment_currencies\":[\"MXN\",\"VND\"],\"transactional_sms_disabled\":true,\"marketing_sms_consent_enabled_at_checkout\":false}"#;

    let compressed = compress(inp.as_bytes())?;
    let depressed = decompress(compressed.as_slice())?;
    println!("before comp size: {}", inp.len());
    println!("comp size: {}", compressed.len());
    assert_eq!(inp.as_bytes(), depressed.as_slice());

    Ok(())
}
