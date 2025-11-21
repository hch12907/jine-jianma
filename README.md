如何运行：

```bash
cargo run --release -- --frequency "beiyu:1.0,zhihu:1.5,tw:0.05" --out output/yuming_b4_z6_t.txt mabiao/yuming_chaifen.dict.yaml
```

命令行参数说明：

```
  --frequency       字频表，可以混合使用多个字频表。默认为 beiyu:0.5,zhihu:0.5。
  --count           字根数量表，记录了一个字有多少个字根。
  --allow           可用编码表，里头列出被额外允许的一些编码。 一行一个编码，# 开头的行会被忽略。
  --predefined      预定义编码表，里头列出被特别制定的编码。 一行一个编码与汉字（用 \t 隔开），# 开头的行会被忽略。
  --out             简码表输出路径。
  --print-candidates
                    打印候选简码。
  --space-jianma    B区键位，默认为 aeiou。代码永远会假设空格是B区键位之一，因此不需要加入空格。
  --b-area          B区码
```
