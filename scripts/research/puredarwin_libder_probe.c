/* Focused Linux reuse probe; no upstream implementation is copied here. */
#include <libDER/libDER.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static unsigned cases, failed;

static void result(const char *name, bool pass, int expected, int actual)
{
    ++cases;
    failed += !pass;
    printf("{\"case\":\"%s\",\"pass\":%s,\"expected\":%d,\"actual\":%d}\n",
           name, pass ? "true" : "false", expected, actual);
}

static void decode(const char *name, DERByte *bytes, size_t len, DERReturn want)
{
    DERItem item = { bytes, len };
    DERDecodedInfo decoded;
    DERReturn got = DERDecodeItem(&item, &decoded);
    result(name, got == want, want, got);
}

static void integer(const char *name, DERByte *bytes, size_t len,
                    DERReturn want, uint32_t value)
{
    DERItem item = { bytes, len };
    uint32_t got_value = 0;
    DERReturn got = DERParseInteger(&item, &got_value);
    result(name, got == want && (got != DR_Success || got_value == value), want, got);
}

static void bitstring(const char *name, DERByte *bytes, size_t len, DERReturn want)
{
    DERItem item = { bytes, len }, output;
    DERByte unused = 0;
    DERReturn got = DERParseBitString(&item, &output, &unused);
    result(name, got == want, want, got);
}

static void encode(const char *name, DERTag tag, const DERByte *content,
                   size_t length, const DERByte *expected, size_t expected_len)
{
    DERByte output[32] = {0};
    DERSize length_out = sizeof(output);
    DERReturn got = DEREncodeItem(tag, length, content, output, &length_out);
    bool pass = got == DR_Success && length_out == expected_len &&
                memcmp(output, expected, expected_len) == 0;
    result(name, pass, (int)expected_len, got == DR_Success ? (int)length_out : -(int)got);
    printf("{\"encoding\":\"%s\",\"hex\":\"", name);
    if (got == DR_Success)
        for (size_t i = 0; i < length_out; ++i) printf("%02x", output[i]);
    puts("\"}");
}

int main(void)
{
    DERByte integer_tlv[] = {0x02, 0x01, 0x01};
    DERByte truncated_tag[] = {0x1f};
    DERByte truncated_length[] = {0x04};
    DERByte truncated_content[] = {0x04, 0x03, 0xaa};
    DERByte indefinite[] = {0x04, 0x80, 0x00, 0x00};
    DERByte nonminimal_length[] = {0x04, 0x81, 0x01, 0xaa};
    DERByte padded_length[] = {0x04, 0x82, 0x00, 0x80};
    DERByte excessive_length[] = {0x04, 0x89, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
    DERByte maximal_length[] = {0x04, 0x88, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff};
    DERByte high_tag[] = {0x1f, 0x1f, 0x00};
    DERByte padded_tag[] = {0x1f, 0x80, 0x1f, 0x00};
    DERByte low_as_high_tag[] = {0x1f, 0x01, 0x00};
    DERByte oversized_tag[] = {0x1f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, 0x00};
    DERByte long_item[131] = {0x04, 0x81, 0x80};
    decode("integer_tlv", integer_tlv, sizeof(integer_tlv), DR_Success);
    decode("long_length_128", long_item, sizeof(long_item), DR_Success);
    decode("truncated_tag", truncated_tag, sizeof(truncated_tag), DR_DecodeError);
    decode("truncated_length", truncated_length, sizeof(truncated_length), DR_DecodeError);
    decode("truncated_content", truncated_content, sizeof(truncated_content), DR_DecodeError);
    decode("indefinite_length", indefinite, sizeof(indefinite), DR_DecodeError);
    decode("nonminimal_length", nonminimal_length, sizeof(nonminimal_length), DR_DecodeError);
    decode("padded_length", padded_length, sizeof(padded_length), DR_DecodeError);
    decode("excessive_length_octets", excessive_length, sizeof(excessive_length), DR_DecodeError);
    decode("maximal_length_truncated", maximal_length, sizeof(maximal_length), DR_DecodeError);
    decode("high_tag_31", high_tag, sizeof(high_tag), DR_Success);
    decode("padded_high_tag", padded_tag, sizeof(padded_tag), DR_DecodeError);
    decode("low_tag_in_high_form", low_as_high_tag, sizeof(low_as_high_tag), DR_DecodeError);
    decode("tag_overflow", oversized_tag, sizeof(oversized_tag), DR_DecodeError);

    DERByte zero[] = {0};
    DERByte positive128[] = {0x00, 0x80};
    DERByte negative[] = {0xff};
    DERByte padded_integer[] = {0x00, 0x01};
    DERByte max_u32[] = {0x00, 0xff, 0xff, 0xff, 0xff};
    DERByte overflow_u32[] = {0x01, 0x00, 0x00, 0x00, 0x00};
    integer("zero_integer", zero, sizeof(zero), DR_Success, 0);
    integer("positive_sign_octet", positive128, sizeof(positive128), DR_Success, 128);
    integer("negative_integer", negative, sizeof(negative), DR_DecodeError, 0);
    integer("padded_integer", padded_integer, sizeof(padded_integer), DR_DecodeError, 0);
    integer("max_u32", max_u32, sizeof(max_u32), DR_Success, UINT32_MAX);
    integer("overflow_u32", overflow_u32, sizeof(overflow_u32), DR_BufOverflow, 0);

    DERByte bit_valid[] = {3, 0xa0};
    DERByte bit_bad_count[] = {8, 0};
    DERByte bit_empty_bad[] = {1};
    DERByte bit_nonzero_unused[] = {3, 0xa7};
    bitstring("valid_bit_string", bit_valid, sizeof(bit_valid), DR_Success);
    bitstring("invalid_unused_count", bit_bad_count, sizeof(bit_bad_count), DR_DecodeError);
    bitstring("empty_nonzero_unused", bit_empty_bad, sizeof(bit_empty_bad), DR_DecodeError);
    /* X.690 (2021) 11.2.1: unused final bits must be zero for DER. */
    bitstring("nonzero_unused_bits", bit_nonzero_unused, sizeof(bit_nonzero_unused), DR_DecodeError);

    DERByte boolean[] = {0xff};
    DERItem boolean_item = {boolean, sizeof(boolean)};
    bool truth = false;
    DERReturn got = DERParseBooleanWithDefault(&boolean_item, false, &truth);
    result("canonical_true", got == DR_Success && truth, DR_Success, got);
    boolean[0] = 1;
    got = DERParseBooleanWithDefault(&boolean_item, false, &truth);
    result("noncanonical_true", got == DR_DecodeError, DR_DecodeError, got);

    DERByte one[] = {1};
    DERByte sequence_wire[] = {0x30, 0x03, 0x02, 0x01, 0x01};
    encode("encode_integer", ASN1_INTEGER, one, sizeof(one), integer_tlv, sizeof(integer_tlv));
    encode("encode_constructed_sequence", ASN1_CONSTR_SEQUENCE, integer_tlv,
           sizeof(integer_tlv), sequence_wire, sizeof(sequence_wire));

    DERItem item = {integer_tlv, sizeof(integer_tlv)};
    DERDecodedInfo decoded;
    got = DERDecodeItem(&item, &decoded);
    result("borrowed_content_view", got == DR_Success && decoded.content.data == integer_tlv + 2 &&
           decoded.content.length == 1, 1, got == DR_Success ? decoded.content.data == integer_tlv + 2 : 0);

    DERByte trailing[] = {0x02, 0x01, 0x01, 0x05, 0x00};
    /* This API intentionally decodes only the first item; whole-input validation is a caller obligation. */
    decode("prefix_decoder_allows_following_tlv", trailing, sizeof(trailing), DR_Success);
    printf("{\"summary\":true,\"cases\":%u,\"failed\":%u}\n", cases, failed);
    return failed == 0 ? 0 : 1;
}
