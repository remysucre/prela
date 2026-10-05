WITH TagStats AS (
    SELECT 
        t.TagName,
        COUNT(DISTINCT p.Id) AS PostCount,
        SUM(p.ViewCount) AS TotalViews,
        SUM(p.Score) AS TotalScore,
        AVG(p.ViewCount) AS AvgViewsPerPost,
        AVG(p.Score) AS AvgScorePerPost
    FROM 
        Tags t
    JOIN 
        Posts p ON p.Tags LIKE '%<' || t.TagName || '>%'
    GROUP BY 
        t.TagName
), UserStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS QuestionsAsked,
        COUNT(DISTINCT CASE WHEN p.PostTypeId = 2 THEN p.Id END) AS AnswersProvided,
        SUM(COALESCE(p.ViewCount, 0)) AS TotalViews,
        SUM(COALESCE(p.Score, 0)) AS TotalScore
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON p.OwnerUserId = u.Id
    GROUP BY 
        u.Id, u.DisplayName
), PopularPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.Score,
        p.CreationDate,
        STRING_AGG(t.TagName, ', ' ORDER BY t.Id) AS TagsList
    FROM 
        Posts p
    JOIN 
        Tags t ON p.Tags LIKE '%<' || t.TagName || '>%'
    WHERE 
        p.ViewCount > 1000
    GROUP BY 
        p.Id, p.Title, p.ViewCount, p.Score, p.CreationDate
)
SELECT 
    ts.TagName,
    ts.PostCount,
    ts.TotalViews,
    ts.TotalScore,
    ts.AvgViewsPerPost,
    ts.AvgScorePerPost,
    us.DisplayName AS TopUser,
    us.QuestionsAsked,
    us.AnswersProvided,
    us.TotalViews AS UserTotalViews,
    us.TotalScore AS UserTotalScore,
    pp.Title AS PopularPostTitle,
    pp.ViewCount AS PopularPostViews,
    pp.Score AS PopularPostScore,
    pp.TagsList
FROM 
    TagStats ts
LEFT JOIN 
    UserStats us ON us.TotalViews = (SELECT MAX(TotalViews) FROM UserStats)
LEFT JOIN 
    PopularPosts pp ON pp.ViewCount = (SELECT MAX(ViewCount) FROM PopularPosts)
ORDER BY 
    ts.PostCount DESC, ts.TotalScore DESC;
