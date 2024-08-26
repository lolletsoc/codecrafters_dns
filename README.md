# DNS for dummies

I created this DNS server over a week at home. It's rare I get to work with
binary protocols in my typical work, so I really enjoyed the process.
I tried out using `package_struct` and then `deku`. However, DNS is
a bit awkward due to label compression - so I ultimately chose a
solution using both `deku` and a hand-coded solution for `QUESTION`
and `ANSWER` resources.
