
WITH UserPostStats AS (
    SELECT 
        u.Id AS UserId,
        COUNT(p.Id) AS PostCount,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS Upvotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS Downvotes,
        COUNT(DISTINCT CASE WHEN p.PostTypeId = 1 THEN p.Id END) AS QuestionCount,
        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswerCount,
        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6 AS AvgPostLifeSpan
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    GROUP BY 
        u.Id
),
TopUsers AS (
    SELECT 
        UserId,
        PostCount,
        Upvotes,
        Downvotes,
        QuestionCount,
        AnswerCount,
        AvgPostLifeSpan,
        RANK() OVER (ORDER BY PostCount DESC) AS RankByPosts
    FROM 
        UserPostStats
)
SELECT 
    UserId,
    PostCount,
    Upvotes,
    Downvotes,
    QuestionCount,
    AnswerCount,
    AvgPostLifeSpan
FROM 
    TopUsers
WHERE 
    RankByPosts <= 10;
