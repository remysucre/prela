WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        p.Tags,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank,
        u.DisplayName AS OwnerDisplayName,
        u.Reputation AS OwnerReputation
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1 
        AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
TopPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.OwnerDisplayName,
        rp.OwnerReputation,
        rp.Score,
        rp.ViewCount,
        rp.Tags
    FROM 
        RankedPosts rp
    WHERE 
        rp.Rank <= 5 
),
TagStatistics AS (
    SELECT 
        t.TagName,
        COUNT(DISTINCT p.Id) AS QuestionCount,
        SUM(p.ViewCount) AS TotalViews,
        AVG(p.Score) AS AverageScore
    FROM 
        Tags t
    JOIN 
        Posts p ON p.Tags LIKE '%' || t.TagName || '%'
    WHERE 
        p.PostTypeId = 1 
    GROUP BY 
        t.TagName
),
UserStatistics AS (
    SELECT 
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS QuestionCount,
        SUM(p.Score) AS TotalScore,
        AVG(p.ViewCount) AS AverageViews
    FROM 
        Users u
    JOIN 
        Posts p ON u.Id = p.OwnerUserId
    WHERE 
        p.PostTypeId = 1 
        AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
    GROUP BY 
        u.DisplayName
)
SELECT 
    tp.Title,
    tp.OwnerDisplayName,
    tp.OwnerReputation,
    tp.Score,
    tp.ViewCount,
    ts.TagName,
    ts.QuestionCount AS TagQuestionCount,
    ts.TotalViews AS TagTotalViews,
    ts.AverageScore AS TagAverageScore,
    us.QuestionCount AS UserQuestionCount,
    us.TotalScore AS UserTotalScore,
    us.AverageViews AS UserAverageViews
FROM 
    TopPosts tp
LEFT JOIN 
    TagStatistics ts ON tp.Tags LIKE '%' || ts.TagName || '%'
LEFT JOIN 
    UserStatistics us ON tp.OwnerDisplayName = us.DisplayName
ORDER BY 
    tp.Score DESC, tp.ViewCount DESC;